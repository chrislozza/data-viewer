use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AccountDailySnapshot {
    pub account_id: String,
    pub date: NaiveDate,
    pub currency: String,
    pub net_liquidating_value: Decimal,
    pub cash_balance: Decimal,
    pub cash_flows: AccountCashFlows,
    pub risk_free_annual: f64,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AccountCashFlows {
    pub deposits: Decimal,
    pub fees: Decimal,
    pub interest: Decimal,
    pub dividends: Decimal,
}
