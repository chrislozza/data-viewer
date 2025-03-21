use axum::{Json, http::StatusCode, response::IntoResponse};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::models::{get_alias, strategy::Strategy};

#[derive(Serialize, Deserialize)]
pub(crate) struct PerformanceRequest {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub is_active: bool,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Performance {
    pub strategy: String,
    pub start_date: NaiveDate,
    pub exit_date: NaiveDate,
    pub start_price: Decimal,
    pub end_price: Decimal,
    pub pnl: Decimal,
    pub roi: Decimal,
}

impl From<&Strategy> for Performance {
    fn from(strategy: &Strategy) -> Self {
        let perf = Performance {
            strategy: get_alias(&strategy.symbol),
            start_date: strategy.entry_time.date_naive(),
            exit_date: strategy.exit_time.date_naive(),
            start_price: strategy.risk.gain.open,
            end_price: strategy.risk.gain.current,
            pnl: strategy.risk.stats.pnl,
            roi: strategy.risk.stats.roi,
        };
        info!("Performance: {}", serde_json::to_string(&perf).unwrap());
        perf
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct PerformanceResponse {
    pub response: Vec<Performance>,
}

impl IntoResponse for PerformanceResponse {
    fn into_response(self) -> axum::response::Response {
        let body = Json(json!({
            "performance": self
        }));

        (StatusCode::OK, body).into_response()
    }
}

impl std::iter::FromIterator<Performance> for PerformanceResponse {
    fn from_iter<T: IntoIterator<Item = Performance>>(iter: T) -> Self {
        PerformanceResponse {
            response: iter.into_iter().collect(),
        }
    }
}
