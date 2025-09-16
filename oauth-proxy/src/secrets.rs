use aws_config::meta::region::RegionProviderChain;
use aws_sdk_lambda::types::error;
use aws_sdk_ssm::{Client, Error};
use tracing::{error, info};

struct Secrets {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
}

impl From<&[String]> for Secrets {
    fn from(secrets: &[String]) -> Self {
        Secrets {
            client_id: secrets[0].to_string(),
            client_secret: secrets[1].to_string(),
            refresh_token: secrets[2].to_string(),
        }
    }
}

pub fn get_region_env_var() -> String {
    std::env::var("REGION").unwrap()
}

async fn get_secrets() -> Result<Secrets, Error> {
    // Load AWS configuration
    let region_provider =
        RegionProviderChain::default_provider().or_else(get_region_env_var().as_str());
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);

    // Get parameter with decryption
    let parameter_ids = ["CLIENT_ID", "CLIENT_SECRET", "REFRESH_TOKEN"];
    let mut secrets = Vec::with_capacity(parameter_ids.len());

    for (idx, id) in parameter_ids.iter().enumerate() {
        let resp = client
            .get_parameter()
            .name("/".to_owned() + id)
            .with_decryption(true)
            .send()
            .await?
            .parameter;

        if let Some(parameter) = resp {
            secrets[idx] = parameter.value().unwrap_or_default().to_string();
            info!("Parameter id: {id}");
        } else {
            error!("Parameter id: {id} not found");
        }
    }

    Ok(Secrets::from(secrets.as_slice()))
}
