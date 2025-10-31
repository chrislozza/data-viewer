use axum::{extract::State, response::IntoResponse};
use std::sync::Arc;

use crate::{
    AppState,
    models::symbol::{Symbol, SymbolResponse},
};

use super::common::AppError;

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
