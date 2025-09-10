use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;

use anyhow::Result;

use super::db_client::DatabaseConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub log_level: String,
}

#[derive(Debug)]
pub struct SettingsReader {}

impl SettingsReader {
    pub fn read_config_file<Settings>(path: &str) -> Result<Settings>
    where
        Settings: for<'de> Deserialize<'de>,
    {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let settings: Settings = serde_json::from_str(&contents)?;
        Ok(settings)
    }
}
