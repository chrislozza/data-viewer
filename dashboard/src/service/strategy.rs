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
    AND (
        (status = 1)
        OR (
            status = 2
            AND exit_time::date >= $2
            AND exit_time::date <= $3
        )
    )
    "#;

    let result = sqlx::query(query)
        .bind(symbol)
        .bind(request.from)
        .bind(request.to)
        .fetch_all(&state.db.pool)
        .await
        .map(|rows| rows.iter().filter_map(Strategy::from_row_safe).collect::<Vec<Strategy>>())
        .map_err(AppError::DatabaseError);

    match result {
        Ok(rows) => StrategyResponse { response: rows }.into_response(),
        Err(e) => e.into_response(),
    }
}
