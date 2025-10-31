use std::fmt;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::DateTime;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use sqlx::Row;
use sqlx::postgres::PgRow;
use sqlx::types::{Json, Uuid};

use crate::models::get_alias;

use super::AssetType;
use super::PriceEffect;
use super::Side;
use super::riskdata::RiskData;
use super::account::AccountDailySnapshot;

#[derive(Debug, Clone, PartialEq, FromRow, Serialize, Deserialize)]
pub(crate) struct Metadata {
    pub local_id: Uuid,
    pub underlying: String,
    pub price_effect: PriceEffect,
    pub asset_type: AssetType,
    pub r#type: StrategyType,
    pub status: Status,
    pub open_price: Decimal,
    pub side: Side,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) enum Status {
    Open,
    #[default]
    Closed,
}

impl From<Status> for i32 {
    fn from(status: Status) -> Self {
        match status {
            Status::Open => 1,
            Status::Closed => 2,
        }
    }
}

impl sqlx::Type<sqlx::Postgres> for Status {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("int4")
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for Status {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> std::result::Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        match self {
            Status::Open => buf.extend(std::iter::once(1)),
            Status::Closed => buf.extend(std::iter::once(2)),
        }
        Ok(sqlx::encode::IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for Status {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let value = <i32 as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(match value {
            1 => Status::Open,
            _ => Status::Closed,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq)]
pub enum StrategyType {
    SingleLeg,
    CreditSpread,
    IronCondor,
    CalendarSpread,
    #[default]
    Other,
}

impl fmt::Display for StrategyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StrategyType::SingleLeg => write!(f, "SingleLeg"),
            StrategyType::CreditSpread => write!(f, "CreditSpread"),
            StrategyType::IronCondor => write!(f, "IronCondor"),
            StrategyType::CalendarSpread => write!(f, "CalendarSpread"),
            StrategyType::Other => write!(f, "Other"),
        }
    }
}

impl sqlx::Type<sqlx::Postgres> for StrategyType {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("text")
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for StrategyType {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let value = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(match value {
            "SingleLeg" => StrategyType::SingleLeg,
            "CreditSpread" => StrategyType::CreditSpread,
            "IronCondor" => StrategyType::IronCondor,
            "CalendarSpread" => StrategyType::CalendarSpread,
            _ => StrategyType::Other,
        })
    }
}

//"strategy": "local_id UUID, symbol VARCHAR, entry_time TIMESTAMPTZ, exit_time TIMESTAMPTZ, status INT, cfg JSON, metadata JSON, risk JSON, orders JSON",
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Strategy {
    pub local_id: Uuid,
    pub symbol: String,
    pub entry_time: DateTime<Utc>,
    pub exit_time: DateTime<Utc>,
    pub status: Status,
    pub meta: Metadata,
    pub risk: RiskData,
    #[serde(default)]
    pub account: AccountDailySnapshot,
}

impl<'r> sqlx::FromRow<'r, PgRow> for Strategy {
    fn from_row(row: &'r PgRow) -> sqlx::Result<Self> {
        Ok(Strategy {
            local_id: row.try_get("local_id")?,
            symbol: get_alias(row.try_get("symbol")?),
            entry_time: row.try_get("entry_time")?,
            exit_time: row.try_get("exit_time")?,
            status: row.try_get("status")?,
            meta: row.try_get::<Json<Metadata>, _>("metadata")?.0,
            risk: row.try_get::<Json<RiskData>, _>("risk")?.0,
            account: row.try_get::<Json<AccountDailySnapshot>, _>("account")?.0,
        })
    }
}

#[derive(Deserialize, Serialize)]
pub struct StrategyResponse {
    pub response: Vec<Strategy>,
}

impl IntoResponse for StrategyResponse {
    fn into_response(self) -> axum::response::Response {
        let body = axum::Json(json!({
            "strategies": self
        }));

        (StatusCode::OK, body).into_response()
    }
}
