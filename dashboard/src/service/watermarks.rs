use axum::{extract::Query, extract::State, response::IntoResponse};
use std::sync::Arc;

use crate::{
    AppState,
    models::watermark::{WatermarkDataPoint, WatermarkRequest, WatermarkResponse},
    models::strategy::Status,
};

use super::common::AppError;

pub(crate) async fn watermarks(
    Query(request): Query<WatermarkRequest>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let year_start = request.to - chrono::Duration::days(365);

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
            use rust_decimal::Decimal;
            use sqlx::Row;
            use std::collections::HashMap;
            use std::str::FromStr;

            let mut watermark_values: Vec<f64> = Vec::new();
            let mut row_data: Vec<(chrono::DateTime<chrono::Utc>, f64)> = Vec::new();

            for row in &rows {
                let exit_time: chrono::DateTime<chrono::Utc> = row.try_get("exit_time").unwrap_or_default();
                let watermark_str: String = row.try_get("watermark").unwrap_or_default();

                if let Ok(watermark) = Decimal::from_str(&watermark_str) {
                    let mut watermark_f64 = watermark.to_string().parse::<f64>().unwrap_or(0.0);

                    if watermark_f64 > 0.0 && watermark_f64 < 1.0 {
                        watermark_f64 *= 100.0;
                    }

                    watermark_values.push(watermark_f64);
                    row_data.push((exit_time, watermark_f64));
                }
            }

            let min_watermark = 20.0;
            let max_watermark = 40.0;

            let mut watermark_ranges: Vec<(f64, f64, String)> = Vec::new();
            let mut current = min_watermark;
            while current < max_watermark {
                let next = current + 1.0;
                let label = format!("{}", current as i32);
                watermark_ranges.push((current, next, label));
                current = next;
            }

            let mut heatmap_data: HashMap<(String, String), i32> = HashMap::new();

            for (exit_time, watermark_f64) in row_data {
                let days_from_start = (exit_time.date_naive() - year_start).num_days();
                let week_number = (days_from_start / 7).max(0).min(51);

                let week_start = year_start + chrono::Duration::days(week_number * 7);
                let time_label = format!("W{:02}-{}", week_number + 1, week_start.format("%m/%d"));

                for (min, max, label) in &watermark_ranges {
                    if watermark_f64 >= *min && watermark_f64 < *max {
                        let key = (time_label.clone(), label.to_string());
                        *heatmap_data.entry(key).or_insert(0) += 1;
                        break;
                    }
                }
            }

            let data: Vec<WatermarkDataPoint> = heatmap_data
                .into_iter()
                .map(|((x, y), value)| WatermarkDataPoint { x, y, value })
                .collect();

            WatermarkResponse {
                data,
                min_watermark,
                max_watermark,
            }
            .into_response()
        }
        Err(e) => e.into_response(),
    }
}
