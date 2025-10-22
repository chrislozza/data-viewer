use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::NaiveDate;
use serde_json::json;
use std::sync::Arc;

use crate::{
    AppState,
    models::{
        performance::{Performance, PerformanceRequest, PerformanceResponse},
        strategy::{Status, Strategy, StrategyResponse},
        symbol::{Symbol, SymbolResponse},
        watermark::{WatermarkDataPoint, WatermarkRequest, WatermarkResponse},
    },
};

enum AppError {
    DatabaseError(sqlx::Error),
    // Add other error types as needed
}

// Implement IntoResponse for your error type
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            ),
        };

        // Create a JSON response with the error details
        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

#[derive(serde::Deserialize, sqlx::Encode)]
pub struct SimpleRequest {
    from: NaiveDate,
    to: NaiveDate,
}
pub(crate) async fn health() -> StatusCode {
    StatusCode::OK
}

pub(crate) async fn symbols(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let query = r#"
    SELECT 
        Distinct(symbol)
    FROM 
        strategy
    "#;

    let result = sqlx::query_as::<_, Symbol>(query)
        .fetch_all(&state.db.pool)
        .await
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => SymbolResponse {
            response: rows.iter().map(Symbol::from).collect(),
        }
        .into_response(),
        Err(e) => e.into_response(),
    }
}

pub(crate) async fn strategy(
    Path(symbol): Path<String>,
    Query(request): Query<SimpleRequest>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let query = r#"
    SELECT
        *
    FROM
        strategy
    WHERE
        symbol = $1
    AND entry_time >= $2
    AND exit_time <= $3
    "#;

    let result = sqlx::query_as::<_, Strategy>(query)
        .bind(symbol)
        .bind(request.from)
        .bind(request.to)
        .fetch_all(&state.db.pool)
        .await
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => StrategyResponse { response: rows }.into_response(),
        Err(e) => e.into_response(),
    }
}

pub(crate) async fn universe(
    Query(request): Query<SimpleRequest>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let query = r#"
    SELECT
        *
    FROM
        strategy
    WHERE
        entry_time >= $1
    AND exit_time <= $2
    "#;

    let result = sqlx::query_as::<_, Strategy>(query)
        .bind(request.from)
        .bind(request.to)
        .fetch_all(&state.db.pool)
        .await
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => StrategyResponse { response: rows }.into_response(),
        Err(e) => e.into_response(),
    }
}

pub(crate) async fn performance(
    Query(request): Query<PerformanceRequest>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let query = r#"
    SELECT
        *
    FROM
        strategy
    WHERE
        entry_time >= $1
    AND exit_time <= $2
    AND status = $3
    "#;

    let status = match request.is_active {
        true => Status::Open,
        false => Status::Closed,
    };

    let result = sqlx::query_as::<_, Strategy>(query)
        .bind(request.from)
        .bind(request.to)
        .bind(Into::<i32>::into(status))
        .fetch_all(&state.db.pool)
        .await
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => PerformanceResponse {
            response: rows.iter().map(Performance::from).collect(),
        }
        .into_response(),
        Err(e) => e.into_response(),
    }
}

pub(crate) async fn watermarks(
    Query(request): Query<WatermarkRequest>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Always use full year range (365 days from request.to going backwards)
    let year_start = request.to - chrono::Duration::days(365);
    
    // Query to get watermarks from closed trades with positive PNL
    let query = r#"
    SELECT
        exit_time,
        (risk->>'stats')::jsonb->>'pnl' as pnl,
        (risk->>'loss')::jsonb->>'watermark' as watermark
    FROM
        strategy
    WHERE
        entry_time >= $1
    AND exit_time <= $2
    AND status = $3
    AND (risk->>'stats')::jsonb->>'pnl' IS NOT NULL
    AND ((risk->>'stats')::jsonb->>'pnl')::numeric > 0
    AND (risk->>'loss')::jsonb->>'watermark' IS NOT NULL
    "#;

    let status = Status::Closed;

    let result = sqlx::query(query)
        .bind(year_start)
        .bind(request.to)
        .bind(Into::<i32>::into(status))
        .fetch_all(&state.db.pool)
        .await
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => {
            use std::collections::HashMap;
            use rust_decimal::Decimal;
            use std::str::FromStr;
            use sqlx::Row;

            // First pass: collect all watermark values to find min/max
            let mut watermark_values: Vec<f64> = Vec::new();
            let mut row_data: Vec<(chrono::DateTime<chrono::Utc>, f64)> = Vec::new();

            for row in &rows {
                let exit_time: chrono::DateTime<chrono::Utc> = row.try_get("exit_time").unwrap_or_default();
                let watermark_str: String = row.try_get("watermark").unwrap_or_default();
                
                if let Ok(watermark) = Decimal::from_str(&watermark_str) {
                    let mut watermark_f64 = watermark.to_string().parse::<f64>().unwrap_or(0.0);
                    
                    // Watermark might be stored as decimal (0.2) instead of percentage (20)
                    if watermark_f64 > 0.0 && watermark_f64 < 1.0 {
                        watermark_f64 *= 100.0;
                    }
                    
                    watermark_values.push(watermark_f64);
                    row_data.push((exit_time, watermark_f64));
                }
            }

            // Fixed watermark range: 20-40
            let min_watermark = 20.0;
            let max_watermark = 40.0;

            // Create watermark ranges with scale of 1
            let mut watermark_ranges: Vec<(f64, f64, String)> = Vec::new();
            let mut current = min_watermark;
            while current < max_watermark {
                let next = current + 1.0;
                let label = format!("{}", current as i32);
                watermark_ranges.push((current, next, label));
                current = next;
            }

            let mut heatmap_data: HashMap<(String, String), i32> = HashMap::new();

            // Second pass: bucket the data by week
            for (exit_time, watermark_f64) in row_data {
                // Calculate week number from year_start
                let days_from_start = (exit_time.date_naive() - year_start).num_days();
                let week_number = (days_from_start / 7).max(0).min(51); // 0-51 for 52 weeks
                
                // Calculate the start date of this week
                let week_start = year_start + chrono::Duration::days(week_number * 7);
                let time_label = format!("W{:02}-{}", week_number + 1, week_start.format("%m/%d"));
                
                // Determine watermark range
                for (min, max, label) in &watermark_ranges {
                    if watermark_f64 >= *min && watermark_f64 < *max {
                        let key = (time_label.clone(), label.to_string());
                        *heatmap_data.entry(key).or_insert(0) += 1;
                        break;
                    }
                }
            }

            // Convert to response format
            let data: Vec<WatermarkDataPoint> = heatmap_data
                .into_iter()
                .map(|((x, y), value)| WatermarkDataPoint { x, y, value })
                .collect();

            WatermarkResponse { 
                data,
                min_watermark,
                max_watermark
            }.into_response()
        }
        Err(e) => e.into_response(),
    }
}
