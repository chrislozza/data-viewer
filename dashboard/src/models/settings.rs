use common::{aws_logging::LoggingConfig, db_client::DatabaseConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
}