use aws_config::meta::region::RegionProviderChain;
use aws_sdk_ssm::types::{Parameter, error::AssociationDoesNotExist};
use aws_sdk_ssm::{Client, Error};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

const CLIENT_ID: &str = "CLIENT_ID";
const REFRESH_TOKEN: &str = "REFRESH_TOKEN";

#[derive(Serialize)]
pub struct Secrets {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
}

impl From<&[String; 3]> for Secrets {
    fn from(secrets: &[String; 3]) -> Self {
        Secrets {
            client_id: secrets[0].to_string(),
            client_secret: secrets[1].to_string(),
            refresh_token: secrets[2].to_string(),
        }
    }
}

async fn get_parameters(key: &str, client: &Client) -> Result<Parameter, Error> {
    let parameter_out = client
        .get_parameter()
        .name("/".to_owned() + key)
        .with_decryption(true)
        .send()
        .await?;

    let parameter = match parameter_out.parameter {
        Some(param) => param,
        None => {
            let builder = AssociationDoesNotExist::builder();
            return Err(Error::AssociationDoesNotExist(
                builder
                    .message(format!("Parameter id: {} not found", key))
                    .build(),
            ));
        }
    };
    Ok(parameter)
}

pub(crate) async fn get_secrets(req_client_id: &str) -> Result<Secrets, Error> {
    // Load AWS configuration
    let region_provider = RegionProviderChain::default_provider();
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);

    let client_id = get_parameters(CLIENT_ID, &client).await?;
    let client_id = client_id.value.unwrap();
    if req_client_id != client_id {
        let builder = AssociationDoesNotExist::builder();
        return Err(Error::AssociationDoesNotExist(
            builder
                .message("Value of id: client_id not found".to_string())
                .build(),
        ));
    }
    let client_secret = get_parameters(req_client_id, &client).await?;
    let refresh_token = get_parameters(REFRESH_TOKEN, &client).await?;

    info!("Got secret items for client_id {req_client_id}");

    Ok(Secrets::from(&[
        client_id,
        client_secret.value.unwrap(),
        refresh_token.value.unwrap(),
    ]))
}
