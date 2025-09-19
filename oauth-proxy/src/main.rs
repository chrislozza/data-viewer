use axum::{body::Body, response::Response};
use common::Init;
use common::logging::StructuredLogging;
use lambda_http::tracing::subscriber::fmt::format;
use lambda_http::{Error, IntoResponse, Request, RequestExt, service_fn};
use serde::Serializer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

mod secrets;

async fn oauth_endpoints(req: Request) -> Result<Response<Body>, Error> {
    info!("Hello Lambda");

    let path = req.uri().path();

    info!(
        "path: {} query: {}",
        path,
        req.query_string_parameters().to_query_string()
    );
    match path {
        "/oauth/ping" => Ok(Response::builder().status(200).body("pong".into())?),
        "/oauth/token" => token(req).await,
        _ => Ok(Response::builder()
            .status(404)
            .body(format!("Endpoint not Found {}", req.uri()).into())?),
    }
}

async fn token(req: Request) -> Result<Response<Body>, Error> {
    let query_params = req.query_string_parameters();
    info!("Query params: {}", query_params.to_query_string());
    let client_id = match query_params.first("client_id") {
        Some(client_id) => client_id,
        None => {
            return Ok(Response::builder()
                .status(404)
                .body("Client Id Not Found".into())?);
        }
    };

    let secrets = match secrets::get_secrets(client_id).await {
        Ok(secrets) => secrets,
        Err(e) => return Ok(Response::builder().status(500).body(e.to_string().into())?),
    };

    Ok(Response::builder()
        .status(200)
        .body(serde_json::to_string(&secrets).unwrap_or_default().into())?)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = Init::structured_logging();
    lambda_http::run(service_fn(oauth_endpoints)).await?;
    Ok(())
}
