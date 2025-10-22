use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::Row;
use sqlx::postgres::PgRow;

pub(super) mod performance;
pub(super) mod riskdata;
pub(super) mod strategy;
pub(super) mod symbol;
pub(super) mod settings;
pub(super) mod watermark;

fn get_alias(symbol: &str) -> String {
    if symbol.starts_with("/") {
        return symbol[..3].to_string();
    }
    symbol.to_string()
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum Side {
    Call,
    Put,
    Netural,
}

impl<'r> FromRow<'r, PgRow> for Side {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let ty: i32 = row.try_get("side")?;
        match ty {
            1 => Ok(Side::Call),
            2 => Ok(Side::Put),
            _ => Ok(Side::Netural),
        }
    }
}

#[derive(Debug, Clone, Default, Copy, PartialEq, Deserialize, Serialize)]
pub enum PriceEffect {
    #[default]
    Credit,
    Debit,
}

impl From<PriceEffect> for i32 {
    fn from(val: PriceEffect) -> Self {
        match val {
            PriceEffect::Credit => 1,
            PriceEffect::Debit => 2,
        }
    }
}

impl From<&str> for PriceEffect {
    fn from(price_effect: &str) -> Self {
        match price_effect {
            "Credit" | "Short" => PriceEffect::Credit,
            "Debit" | "Long" => PriceEffect::Debit,
            _ => panic!("Unknown price effect"),
        }
    }
}

impl fmt::Display for PriceEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let price_effect = match self {
            PriceEffect::Credit => String::from("Credit"),
            PriceEffect::Debit => String::from("Debit"),
        };
        write!(f, "{price_effect}")
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq)]
pub enum AssetType {
    #[default]
    Equity,
    EquityOption,
    Future,
    FutureOption,
}

impl FromRow<'_, PgRow> for AssetType {
    fn from_row(row: &PgRow) -> sqlx::Result<Self> {
        sqlx::Result::Ok(match row.try_get("asset_type")? {
            1 => AssetType::Equity,
            2 => AssetType::EquityOption,
            3 => AssetType::Future,
            4 => AssetType::FutureOption,
            _ => panic!("Unknown AssetType"),
        })
    }
}

impl From<AssetType> for i32 {
    fn from(val: AssetType) -> Self {
        match val {
            AssetType::Equity => 1,
            AssetType::EquityOption => 2,
            AssetType::Future => 3,
            AssetType::FutureOption => 4,
        }
    }
}

impl From<&str> for AssetType {
    fn from(instrument_type: &str) -> Self {
        match instrument_type {
            "Equity" => AssetType::Equity,
            "Future" => AssetType::Future,
            "Equity Option" => AssetType::EquityOption,
            "Future Option" => AssetType::FutureOption,
            _ => panic!("Unsupported Type"),
        }
    }
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let instrument_type = match self {
            AssetType::Equity => String::from("Equity"),
            AssetType::Future => String::from("Future"),
            AssetType::EquityOption => String::from("EquityOption"),
            AssetType::FutureOption => String::from("FutureOption"),
        };
        write!(f, "{instrument_type}")
    }
}

impl AssetType {
    pub fn get_asset_type(instrument_type: &str) -> AssetType {
        match instrument_type {
            "Equity" => AssetType::Equity,
            "Future" => AssetType::Future,
            "Equity Option" => AssetType::EquityOption,
            "Future Option" => AssetType::FutureOption,
            _ => panic!("Unsupported Type"),
        }
    }
}
