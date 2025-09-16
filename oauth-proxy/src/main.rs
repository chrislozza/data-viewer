use axum::{body::Body, response::Response};
use lambda_http::{Error, IntoResponse, Request, RequestExt, service_fn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod secrets;

async fn oauth_endpoints(req: Request) -> Result<Response<Body>, Error> {
    let path = req.uri().path();

    println!("path: {}", path);

    Ok(Response::builder().status(200).body("Hello World".into())?)

    // match path {
    //     "/authorise" => authorise(req).await,
    //     "/callback" => callback(req).await,
    //     _ => Ok(Response::builder().status(404).body("Not Found".into())?),
    // }
}

async fn token() -> Result<Response<Body>, Error> {
    Ok(Response::builder().status(404).body("Not Found".into())?)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_http::run(service_fn(oauth_endpoints)).await?;
    Ok(())
}
