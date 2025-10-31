use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// Hard-coded base capital for return normalization
pub(crate) const BASE_CAPITAL: f64 = 5000.0;

#[derive(Serialize, Deserialize)]
pub(crate) struct MetricsRequest {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct DrawdownMetrics {
    pub max_dd_abs: Decimal,
    pub max_dd_pct_base: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trough_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_days: Option<i64>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct SharpeMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharpe: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean_daily: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vol_daily: Option<f64>,
    pub rf_annual: f64,
    pub sample_days: usize,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct ProfitFactorMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profit_factor: Option<f64>,
    pub gross_profit: Decimal,
    pub gross_loss: Decimal,
    pub wins: usize,
    pub losses: usize,
    pub trade_count: usize,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct ExpectancyMetrics {
    pub expectancy_usd: Decimal,
    pub median_usd: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub win_rate: Option<f64>,
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub trade_count: usize,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct RecoveryFactorMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_factor: Option<f64>,
    pub net_profit: Decimal,
    pub reference_max_dd: Decimal,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct MetricsResponseBody {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub drawdown: DrawdownMetrics,
    pub sharpe: SharpeMetrics,
    pub expectancy: ExpectancyMetrics,
    pub recovery: RecoveryFactorMetrics,
    pub profit_factor: ProfitFactorMetrics,
}
