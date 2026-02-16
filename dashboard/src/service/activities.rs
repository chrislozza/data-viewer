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
    pub strategy_type: Option<String>, // Short strategy type (CS, IC, etc.)
    pub symbol: Option<String>, // Symbol without /USD
}

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    pub limit: Option<usize>,
}

pub(crate) async fn activities(
    Query(params): Query<ActivityQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(6);
    
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
    
    // Fetch open orders
    let open_orders_result = fetch_open_orders(&state.db, limit).await;
    let open_order_activities = match open_orders_result {
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
        .chain(open_order_activities.into_iter())
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
            (risk->>'stats')::jsonb->>'fee' as fee,
            (metadata->>'type') as strategy_type,
            (risk->>'gain')::jsonb->>'open' as entry_price,
            (risk->>'gain')::jsonb->>'current' as exit_price
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
            let strategy_type: String = row.try_get("strategy_type").unwrap_or_default();
            let entry_price: String = row.try_get("entry_price").unwrap_or_default();
            let exit_price: String = row.try_get("exit_price").unwrap_or_default();
            
            let pnl = pnl_str.parse::<f64>().unwrap_or(0.0);
            let fee = fee_str.parse::<f64>().unwrap_or(0.0);
            let net_pnl = pnl - fee;
            
            let entry = entry_price.parse::<f64>().unwrap_or(0.0);
            let exit = exit_price.parse::<f64>().unwrap_or(0.0);
            
            // Get shortened strategy type
            let strategy_short = match strategy_type.trim() {
                "SingleLeg" => "SL",
                "CreditSpread" => "CS",
                "IronCondor" => "IC",
                "CalendarSpread" => "CL",
                _ => "OT",
            };
            
            let details = format!("E:{:.2} X:{:.2} (${:.2})", entry, exit, net_pnl);
            
            activities.push(Activity {
                timestamp,
                event_type: "trade".to_string(),
                description: "trade executed".to_string(),
                details: Some(details),
                value: Some(net_pnl),
                strategy_type: Some(strategy_short.to_string()),
                symbol: Some(ticker.to_uppercase()),
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
            (risk->>'loss')::jsonb->>'watermark' as watermark_level,
            (metadata->>'type') as strategy_type,
            (risk->>'gain')::jsonb->>'open' as entry_price,
            (risk->>'gain')::jsonb->>'current' as exit_price
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
            let _watermark_str: String = row.try_get("watermark_level").unwrap_or_default();
            let strategy_type: String = row.try_get("strategy_type").unwrap_or_default();
            let entry_price: String = row.try_get("entry_price").unwrap_or_default();
            let exit_price: String = row.try_get("exit_price").unwrap_or_default();
            
            let pnl = pnl_str.parse::<f64>().unwrap_or(0.0);
            let fee = fee_str.parse::<f64>().unwrap_or(0.0);
            let net_pnl = pnl - fee;
            
            let entry = entry_price.parse::<f64>().unwrap_or(0.0);
            let exit = exit_price.parse::<f64>().unwrap_or(0.0);
            
            // Get shortened strategy type
            let strategy_short = match strategy_type.trim() {
                "SingleLeg" => "SL",
                "CreditSpread" => "CS",
                "IronCondor" => "IC",
                "CalendarSpread" => "CL",
                _ => "OT",
            };
            
            let details = format!("E:{:.2} X:{:.2} (${:.2})", entry, exit, net_pnl);
            
            activities.push(Activity {
                timestamp,
                event_type: "watermark".to_string(),
                description: if net_pnl < 0.0 { "loss".to_string() } else { "profit".to_string() },
                details: Some(details),
                value: Some(net_pnl),
                strategy_type: Some(strategy_short.to_string()),
                symbol: Some(ticker.to_uppercase()),
            });
        }
    }
    
    Ok(activities)
}

async fn fetch_open_orders(db: &DBClient, limit: usize) -> Result<Vec<Activity>, String> {
    let query = r#"
        SELECT 
            entry_time as timestamp,
            symbol as ticker,
            (risk->>'gain')::jsonb->>'open' as entry_price,
            (risk->>'gain')::jsonb->>'current' as current_price,
            (metadata->>'quantity') as quantity,
            (metadata->>'type') as strategy_type,
            (metadata->>'price_effect') as price_effect
        FROM strategy 
        WHERE status = 1
        ORDER BY entry_time DESC 
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
            let entry_price: String = row.try_get("entry_price").unwrap_or_default();
            let current_price: String = row.try_get("current_price").unwrap_or_default();
            let _quantity: String = row.try_get("quantity").unwrap_or_default();
            let strategy_type: String = row.try_get("strategy_type").unwrap_or_default();
            let price_effect: String = row.try_get("price_effect").unwrap_or_default();
            
            let entry = entry_price.parse::<f64>().unwrap_or(0.0);
            let current = current_price.parse::<f64>().unwrap_or(0.0);
            
            // Calculate unrealized PnL based on price effect
            let unrealized_pnl = if entry > 0.0 && current > 0.0 {
                if price_effect == "Credit" {
                    // For credit spreads: profit when price decreases
                    Some(entry - current)
                } else {
                    // For debit spreads: profit when price increases
                    Some(current - entry)
                }
            } else {
                None
            };
            
            // Get shortened strategy type from the metadata string
            let strategy_short = match strategy_type.trim() {
                "SingleLeg" => "SL",
                "CreditSpread" => "CS",
                "IronCondor" => "IC",
                "CalendarSpread" => "CL",
                _ => "OT",
            };
            
            let details = if let Some(pnl) = unrealized_pnl {
                Some(format!("E:{:.2} C:{:.2} (${:.2})", entry, current, pnl))
            } else {
                Some(format!("E:{:.2}", entry))
            };
            
            activities.push(Activity {
                timestamp,
                event_type: "open_order".to_string(),
                description: "open".to_string(),
                details,
                value: unrealized_pnl,
                strategy_type: Some(strategy_short.to_string()),
                symbol: Some(ticker.to_uppercase()),
            });
        }
    }
    
    Ok(activities)
}
