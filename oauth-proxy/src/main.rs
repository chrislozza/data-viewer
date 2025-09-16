use axum::{body::Body, response::Response};
use lambda_http::{Error, IntoResponse, Request, RequestExt, service_fn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

async fn authorise(req: Request) -> Result<Response<Body>, Error> {
    let query = req.uri().query().unwrap_or("");

    let url = format!("https://my.tastytrade.com/auth.html?{}", query);

    Ok(Response::builder()
        .status(200)
        .header("Location", url)
        .body(Body::empty())?)
}

async fn callback(req: Request) -> Result<Response<Body>, Error> {
    let query_params = req.uri().query().unwrap_or("");
    let params: HashMap<_, _> = url::form_urlencoded::parse(query_params.as_bytes()).collect();

    let code = params.get("code").ok_or("Missing code parameter")?;
    let state = params.get("state").ok_or("Missing state parameter")?;

    // verify_state(state)?;

    // let tokens = exchange_code_for_tokens(code).await?;

    // store_tokens(extract_session_id(state), &tokens).await?;

    // 5. Show success page to user
    Ok(Response::builder()
        .status(200)
        .body("Authorization successful! You can return to the application.".into())?)
}

async fn exchange_code_for_tokens(code: &str) -> Result<TokenResponse, Error> {
    let client = reqwest::Client::new();

    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        // ("client_id", CLIENT_ID),
        // ("client_secret", CLIENT_SECRET),
        // ("redirect_uri", REDIRECT_URI),
    ];

    let response = client
        .post("https://api.tastyworks.com/oauth/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Token request failed: {}", error_text).into());
    }

    // Parse the token response
    let token_response: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    Ok(token_response)
}

#[derive(Deserialize, Serialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: String,
    expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_token: Option<String>,
}

async fn token() -> Result<Response<Body>, Error> {
    Ok(Response::builder().status(404).body("Not Found".into())?)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_http::run(service_fn(oauth_endpoints)).await?;
    Ok(())
}
