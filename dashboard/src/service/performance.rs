use axum::{extract::Query, extract::State, response::IntoResponse};
use std::sync::Arc;

use crate::{
    AppState,
    models::{
        performance::{Performance, PerformanceRequest, PerformanceResponse},
        strategy::{Status, Strategy},
    },
};

use super::common::AppError;

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
