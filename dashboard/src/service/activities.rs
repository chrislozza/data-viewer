use axum::{extract::{Query, State}, Json, response::IntoResponse};
use chrono::{DateTime, Utc};
use common::db_client::DBClient;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::Row;
use std::sync::Arc;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Activity {
    pub timestamp: DateTime<Utc>,
    pub event_type: String, // "trade", "watermark", "strategy"
    pub description: String,
    pub details: Option<String>,
    pub value: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    pub limit: Option<usize>,
}

pub(crate) async fn activities(
    Query(params): Query<ActivityQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(5);
    
    // Fetch recent trades
    let trade_result = fetch_recent_trades(&state.db, limit).await;
    let trade_activities = match trade_result {
        Ok(activities) => activities,
        Err(e) => {
            let body = Json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, body).into_response();
        }
    };
    
    // Fetch recent watermark hits
    let watermark_result = fetch_recent_watermarks(&state.db, limit).await;
    let watermark_activities = match watermark_result {
        Ok(activities) => activities,
        Err(e) => {
            let body = Json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, body).into_response();
        }
    };
    
    // Combine and sort by timestamp
    let mut all_activities: Vec<Activity> = trade_activities
        .into_iter()
        .chain(watermark_activities.into_iter())
        .collect();
    
    all_activities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    
    // Limit to requested number
    all_activities.truncate(limit);
    
    let response = Json(serde_json::json!({ "activities": all_activities }));
    (axum::http::StatusCode::OK, response).into_response()
}

async fn fetch_recent_trades(db: &DBClient, limit: usize) -> Result<Vec<Activity>, String> {
    let query = r#"
        SELECT 
            exit_time as timestamp,
            symbol as ticker,
            (risk->>'stats')::jsonb->>'pnl' as pnl,
            (risk->>'stats')::jsonb->>'fee' as fee
        FROM strategy 
        WHERE status = 2
        AND (risk->>'loss')::jsonb->>'watermark' IS NULL
        ORDER BY exit_time DESC 
        LIMIT $1
    "#;
    
    let rows = sqlx::query(query)
        .bind(limit as i32)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?;
    
    let mut activities = Vec::new();
    for row in rows {
        if let Ok(timestamp) = row.try_get::<DateTime<Utc>, _>("timestamp") {
            let ticker: String = row.try_get("ticker").unwrap_or_default();
            let pnl_str: String = row.try_get("pnl").unwrap_or_default();
            let fee_str: String = row.try_get("fee").unwrap_or_default();
            
            let pnl = pnl_str.parse::<f64>().unwrap_or(0.0);
            let fee = fee_str.parse::<f64>().unwrap_or(0.0);
            let net_pnl = pnl - fee;
            
            activities.push(Activity {
                timestamp: timestamp,
                event_type: "trade".to_string(),
                description: format!("{}/USD trade executed", ticker.to_uppercase()),
                details: Some(format!("{:+.2}", net_pnl)),
                value: Some(net_pnl),
            });
        }
    }
    
    Ok(activities)
}

async fn fetch_recent_watermarks(db: &DBClient, limit: usize) -> Result<Vec<Activity>, String> {
    let query = r#"
        SELECT 
            exit_time as timestamp,
            symbol as ticker,
            (risk->>'stats')::jsonb->>'pnl' as pnl,
            (risk->>'stats')::jsonb->>'fee' as fee,
            (risk->>'loss')::jsonb->>'watermark' as watermark_level
        FROM strategy 
        WHERE status = 2
        AND (risk->>'loss')::jsonb->>'watermark' IS NOT NULL
        ORDER BY exit_time DESC 
        LIMIT $1
    "#;
    
    let rows = sqlx::query(query)
        .bind(limit as i32)
        .fetch_all(&db.pool)
        .await
        .map_err(|e| e.to_string())?;
    
    let mut activities = Vec::new();
    for row in rows {
        if let Ok(timestamp) = row.try_get::<DateTime<Utc>, _>("timestamp") {
            let ticker: String = row.try_get("ticker").unwrap_or_default();
            let pnl_str: String = row.try_get("pnl").unwrap_or_default();
            let fee_str: String = row.try_get("fee").unwrap_or_default();
            let watermark_str: String = row.try_get("watermark_level").unwrap_or_default();
            
            let pnl = pnl_str.parse::<f64>().unwrap_or(0.0);
            let fee = fee_str.parse::<f64>().unwrap_or(0.0);
            let net_pnl = pnl - fee;
            
            // Determine if this was a stop loss or profitable exit
            let (description, details) = if net_pnl < 0.0 {
                (format!("{}/USD loss target hit", ticker.to_uppercase()), 
                 format!("{:.2}", net_pnl))
            } else {
                (format!("{}/USD profit target hit", ticker.to_uppercase()), 
                 format!("{:+.2}", net_pnl))
            };
            
            activities.push(Activity {
                timestamp: timestamp,
                event_type: "watermark".to_string(),
                description: description,
                details: Some(details),
                value: Some(net_pnl),
            });
        }
    }
    
    Ok(activities)
}
