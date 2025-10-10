use axum::{body::Body, response::Response};
use lambda_http::{Error, Request, RequestExt, http::StatusCode, service_fn};
use common::aws_logging;
use tracing::info;

use crate::secrets::Endpoint;

mod secrets;
mod settings;

async fn oauth_endpoints(req: Request) -> Result<Response<Body>, Error> {
    info!("Hello Lambda");

    let path = req.uri().path();

    let res = check_headers(&req).await?;
    if res.status() != StatusCode::ACCEPTED {
        return Ok(res);
    }

    info!(
        "path: {} query: {}",
        path,
        req.query_string_parameters().to_query_string()
    );
    match path {
        "/oauth/ping" => Ok(Response::builder()
            .status(StatusCode::OK)
            .body("pong".into())?),
        "/oauth/token" => token(req).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(format!("Endpoint not Found {}", req.uri()).into())?),
    }
}

async fn check_headers(req: &Request) -> Result<Response<Body>, Error> {
    let headers = req.headers();
    let api_key = match headers.get("x-api-key") {
        Some(api_key) => api_key,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("API Key Not Found".into())?);
        }
    };
    let stored_api_key = secrets::aws_api_key().await?;
    if stored_api_key.ne(api_key) {
        Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("Unauthorized key".into())?)
    } else {
        Ok(Response::builder()
            .status(StatusCode::ACCEPTED)
            .body("Authorized".into())?)
    }
}

async fn token(req: Request) -> Result<Response<Body>, Error> {
    let query_params = req.query_string_parameters();
    info!("Query params: {}", query_params.to_query_string());
    let client_id = match query_params.first("client_id") {
        Some(client_id) => client_id,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Client Id Not Found".into())?);
        }
    };

    let endpoint = match query_params.first("endpoint") {
        Some(client_id) => client_id,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Endpoint Not Found".into())?);
        }
    };

    let secrets = match secrets::get_secrets(client_id, Endpoint::from(endpoint)).await {
        Ok(secrets) => secrets,
        Err(e) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(e.to_string().into())?);
        }
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(serde_json::to_string(&secrets).unwrap_or_default().into())?)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = aws_logging::init_cloudwatch_logger(&aws_logging::LoggingConfig { 
        log_group: "trading-tools".to_string(), 
        log_stream: "oauth-proxy".to_string(), 
        level: "INFO".to_string() 
    });
    lambda_http::run(service_fn(oauth_endpoints)).await?;
    Ok(())
}
