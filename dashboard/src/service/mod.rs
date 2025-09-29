use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::NaiveDate;
use futures::StreamExt;
use futures::TryStreamExt;
use serde_json::json;
use std::sync::Arc;

use crate::{
    AppState,
    models::{
        performance::{Performance, PerformanceRequest, PerformanceResponse},
        strategy::{Status, Strategy, StrategyResponse},
        symbol::{Symbol, SymbolResponse},
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
