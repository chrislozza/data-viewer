use axum::{extract::Query, extract::State, response::IntoResponse};
use tracing::info;
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
        exit_time::date >= $1
    AND exit_time::date <= $2
    AND status = $3
    "#;

    let status = match request.is_active {
        true => Status::Open,
        false => Status::Closed,
    };

    let result = sqlx::query(query)
        .bind(request.from)
        .bind(request.to)
        .bind(Into::<i32>::into(status))
        .fetch_all(&state.db.pool)
        .await
        .map(|rows| rows.iter().filter_map(Strategy::from_row_safe).collect::<Vec<Strategy>>())
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => {
            let perf = PerformanceResponse {
                response: rows.iter().map(Performance::from).collect(),
            };
            info!("Performance: {}", serde_json::to_string(&perf).unwrap());
            perf.into_response()
        }
        Err(e) => e.into_response(),
    }
}
