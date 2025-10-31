use axum::{extract::{Path, Query, State}, response::IntoResponse};
use std::sync::Arc;

use crate::{
    AppState,
    models::strategy::{Strategy, StrategyResponse},
};

use super::common::{AppError, SimpleRequest};

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
