use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::get,
};
use clap::Parser;
use db_client::DBClient;
use serde_json::to_string;
use std::sync::Arc;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::info;

mod db_client;
mod logging;
mod models;
mod service;
mod settings;

use settings::Config;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    settings: String,
}

fn graceful_shutdown(is_graceful_shutdown: &mut bool, shutdown_signal: &CancellationToken) {
    *is_graceful_shutdown = true;
    info!("Graceful shutdown initiated");
    shutdown_signal.cancel();
}

struct AppState {
    db: DBClient,
}

#[tokio::main]
async fn main() {
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();

    let cmdline_args = Args::parse();
    let settings = match Config::read_config_file(cmdline_args.settings.as_str()) {
        Err(val) => {
            println!("Settings file: {} error: {}", cmdline_args.settings, val);
            std::process::exit(1);
        }
        Ok(val) => val,
    };

    let cancel_token = CancellationToken::new();
    let _ = logging::Logging::new(&settings.log_level)
        .await
        .expect("Failed to start logging");

    let version = env!("CARGO_PKG_VERSION");

    info!("___/********Data Viewer v{}********\\___", version);

    info!(
        "Settings: {}",
        &to_string(&settings).expect("Failed to parse settings to json")
    );

    let db = db_client::startup_db(&settings).await;

    let state = Arc::new(AppState { db });

    let base_url = "http://localhost:8000";

    let cors = CorsLayer::new()
        .allow_origin(base_url.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET])
        .allow_headers(Any);

    let app = Router::new()
        .route("/symbols", get(service::symbols))
        .route("/strategy/{symbol}", get(service::strategy))
        .route("/universe", get(service::universe))
        .route("/performance", get(service::performance))
        .with_state(state)
        .layer(cors)
        .fallback_service(ServeDir::new("frontend"));

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();

    let mut is_graceful_shutdown = false;
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    break;
                }
                _ = sigterm.recv() => {
                    graceful_shutdown(&mut is_graceful_shutdown, &cancel_token);
                }
                _ = signal::ctrl_c() => {
                    graceful_shutdown(&mut is_graceful_shutdown, &cancel_token);
                }
            }
        }
    });
}
