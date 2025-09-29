use anyhow::Result;
use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug)]
pub struct SettingsReader {}

impl SettingsReader {
    /// Read settings from a local file
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

    /// Read settings from S3 bucket
    pub async fn read_config_from_s3<Settings>(
        bucket_name: &str,
        object_key: &str,
    ) -> Result<Settings>
    where
        Settings: for<'de> Deserialize<'de>,
    {
        crate::s3_config::read_json_config_from_s3(bucket_name, object_key).await
    }
}
