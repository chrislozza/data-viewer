use anyhow::{Context, Result};
use serde::Deserialize;

/// S3 configuration reader for application settings
pub struct S3ConfigReader {
    client: aws_sdk_s3::Client,
}

impl S3ConfigReader {
    /// Create a new S3ConfigReader with AWS credentials from environment
    pub async fn new() -> Result<Self> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);
        Ok(Self { client })
    }

    /// Create a new S3ConfigReader with a custom AWS config
    pub fn with_config(config: &aws_config::SdkConfig) -> Self {
        let client = aws_sdk_s3::Client::new(config);
        Self { client }
    }

    /// Read and parse a JSON configuration file from S3
    pub async fn read_json_config<T>(&self, bucket: &str, key: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .context(format!(
                "Failed to fetch configuration from S3: s3://{}/{}",
                bucket, key
            ))?;

        let body = response.body.collect().await.context(format!(
            "Failed to read response body from s3://{}/{}",
            bucket, key
        ))?;

        let contents = String::from_utf8(body.into_bytes().to_vec())
            .context("Failed to parse S3 object as UTF-8")?;

        let config: T = serde_json::from_str(&contents)
            .context("Failed to parse configuration JSON from S3")?;

        Ok(config)
    }

    /// Read raw string content from S3
    pub async fn read_string(&self, bucket: &str, key: &str) -> Result<String> {
        let response = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .context(format!(
                "Failed to fetch content from S3: s3://{}/{}",
                bucket, key
            ))?;

        let body = response.body.collect().await.context(format!(
            "Failed to read response body from s3://{}/{}",
            bucket, key
        ))?;

        let contents = String::from_utf8(body.into_bytes().to_vec())
            .context("Failed to parse S3 object as UTF-8")?;

        Ok(contents)
    }

    /// Read raw bytes from S3
    pub async fn read_bytes(&self, bucket: &str, key: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .context(format!(
                "Failed to fetch content from S3: s3://{}/{}",
                bucket, key
            ))?;

        let body = response.body.collect().await.context(format!(
            "Failed to read response body from s3://{}/{}",
            bucket, key
        ))?;

        Ok(body.into_bytes().to_vec())
    }
}

/// Convenience function to read JSON config from S3 without creating a reader
pub async fn read_json_config_from_s3<T>(bucket: &str, key: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let reader = S3ConfigReader::new().await?;
    reader.read_json_config::<T>(bucket, key).await
}
