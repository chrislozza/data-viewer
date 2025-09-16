use crate::logging::Logging;
use anyhow::Result;

pub mod db_client;
pub mod logging;
pub mod settings;

pub struct Init {}

impl Init {
    pub async fn logging(log_level: &str) -> Result<Logging> {
        Logging::new(log_level).await
    }
}
