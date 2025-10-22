use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkRequest {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkDataPoint {
    pub x: String,           // Time period label
    pub y: String,           // Watermark range label
    pub value: i32,          // Count of trades in this bucket
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkResponse {
    pub data: Vec<WatermarkDataPoint>,
    pub min_watermark: f64,
    pub max_watermark: f64,
}

impl IntoResponse for WatermarkResponse {
    fn into_response(self) -> axum::response::Response {
        let body = axum::Json(json!({
            "watermarks": self.data,
            "min_watermark": self.min_watermark,
            "max_watermark": self.max_watermark
        }));

        (StatusCode::OK, body).into_response()
    }
}
