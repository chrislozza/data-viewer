use axum::{http::StatusCode, response::IntoResponse, Json};
use chrono::NaiveDate;
use serde_json::json;

#[derive(serde::Deserialize, sqlx::Encode)]
pub struct SimpleRequest {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

pub enum AppError {
    DatabaseError(sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            ),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
