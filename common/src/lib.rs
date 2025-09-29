use tracing::info;

use crate::settings::SettingsReader; 

pub mod db_client;
pub mod aws_logging;
pub mod settings;
pub mod s3_config;
pub mod parameter_store;

const S3_BUCKET_TRADING: &str = "S3_BUCKET_TRADING";

pub async fn load_settings_from_s3<T>(param_name: &str) -> T
where
    T: for<'de> serde::Deserialize<'de>, 
{
    let bucket = match parameter_store::get_parameter(S3_BUCKET_TRADING, false).await {
        Ok(val) => val,
        Err(e) => {
            println!("Failed to get S3_CONFIG_BUCKET from env or SSM: {}", e);
            std::process::exit(1);
        }
    };
    
    info!("Loading settings from S3: s3://{}/{}", bucket, param_name);
    match SettingsReader::read_config_from_s3::<T>(&bucket, param_name).await {
        Err(val) => {
            println!("Failed to load settings from S3: {}", val);
            std::process::exit(1);
        }
        Ok(val) => val,
    }
}