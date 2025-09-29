use crate::logging::{Logging, StructuredLogging};
use anyhow::Result;

pub mod db_client;
pub mod logging;
pub mod aws_logging;
pub mod settings;

pub struct Init {}

impl Init {
    pub fn logging(log_level: &str) -> Result<Logging> {
        Logging::new(log_level)
    }

    pub fn structured_logging() -> Result<StructuredLogging> {
        StructuredLogging::new()
    }
}
