use anyhow::{Context, Result};
use aws_sdk_ssm::Client;

/// AWS Systems Manager Parameter Store client
pub struct ParameterStore {
    client: Client,
}

impl ParameterStore {
    /// Create a new ParameterStore client with AWS credentials from environment
    pub async fn new() -> Result<Self> {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);
        Ok(Self { client })
    }

    /// Create a new ParameterStore client with a custom AWS config
    pub fn with_config(config: &aws_config::SdkConfig) -> Self {
        let client = Client::new(config);
        Self { client }
    }

    /// Get a parameter value by name
    /// The name should start with "/" (e.g., "/my-app/config")
    pub async fn get_parameter(&self, name: &str, decrypt: bool) -> Result<String> {
        let response = self
            .client
            .get_parameter()
            .name(name)
            .with_decryption(decrypt)
            .send()
            .await
            .context(format!("Failed to get parameter: {}", name))?;

        let parameter = response
            .parameter
            .context(format!("Parameter not found: {}", name))?;

        let value = parameter
            .value
            .context(format!("Parameter has no value: {}", name))?;

        Ok(value)
    }

    /// Get multiple parameters by names
    pub async fn get_parameters(
        &self,
        names: &[String],
        decrypt: bool,
    ) -> Result<Vec<(String, String)>> {
        let response = self
            .client
            .get_parameters()
            .set_names(Some(names.to_vec()))
            .with_decryption(decrypt)
            .send()
            .await
            .context("Failed to get parameters")?;

        let parameters = response.parameters.unwrap_or_default();

        let results = parameters
            .into_iter()
            .filter_map(|p| {
                let name = p.name?;
                let value = p.value?;
                Some((name, value))
            })
            .collect();

        Ok(results)
    }

    /// Get a parameter value by name with a prefix automatically added
    /// Example: get_parameter_with_prefix("config", "/my-app") -> "/my-app/config"
    pub async fn get_parameter_with_prefix(
        &self,
        name: &str,
        prefix: &str,
        decrypt: bool,
    ) -> Result<String> {
        let full_name = format!("{}/{}", prefix.trim_end_matches('/'), name);
        self.get_parameter(&full_name, decrypt).await
    }
}

/// Convenience function to get a parameter without creating a client
pub async fn get_parameter(name: &str, decrypt: bool) -> Result<String> {
    let store = ParameterStore::new().await?;
    store.get_parameter(name, decrypt).await
}

/// Convenience function to get a parameter with a prefix
pub async fn get_parameter_with_prefix(
    name: &str,
    prefix: &str,
    decrypt: bool,
) -> Result<String> {
    let store = ParameterStore::new().await?;
    store.get_parameter_with_prefix(name, prefix, decrypt).await
}
