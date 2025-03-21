
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use sqlx::postgres::PgRow;

use crate::models::get_alias;

#[derive(Serialize, Deserialize)]
pub(crate) struct Symbol {
    pub name: String,
}

impl<'r> sqlx::FromRow<'r, PgRow> for Symbol {
    fn from_row(row: &'r PgRow) -> sqlx::Result<Self> {
        Ok(Symbol {
            name: row.try_get("symbol")?,
        })
    }
}

impl From<&Symbol> for Symbol {
    fn from(symbol: &Symbol) -> Self {
        Symbol {
            name: get_alias(&symbol.name),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SymbolResponse {
    pub response: Vec<Symbol>,
}

impl IntoResponse for SymbolResponse {
    fn into_response(self) -> axum::response::Response {
        let body = Json(json!({
            "symbols": self
        }));

        (StatusCode::OK, body).into_response()
    }
}
