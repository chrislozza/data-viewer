use axum::{extract::Query, extract::State, response::IntoResponse};
use std::sync::Arc;

use crate::{
    AppState,
    models::strategy::{Strategy, StrategyResponse},
};

use super::common::SimpleRequest;
use super::common::AppError;

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
