use super::Side;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub(crate) struct Gain {
    pub open: Decimal,
    pub current: Decimal,
    pub target: Decimal,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub(crate) struct Loss {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lower: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upper: Option<Decimal>,
    pub target: Decimal,
    pub watermark: Decimal,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub(crate) struct Stats {
    pub pnl: Decimal,
    pub roi: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct RiskData {
    pub side: Side,
    #[serde(default)]
    pub gain: Gain,
    #[serde(default)]
    pub loss: Loss,
    #[serde(default)]
    pub stats: Stats,
}
