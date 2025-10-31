use axum::{
    Router,
    routing::get,
};
use clap::Parser;
use common::{aws_logging, db_client::{self, DBClient}, load_settings_from_s3, settings::SettingsReader};
use models::settings::Settings;
use serde_json::to_string;
use std::sync::Arc;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

mod models;
mod service;

const S3_STORED_SETTINGS: &str = "settings.json";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    settings: Option<String>,
    
    #[arg(long)]
    frontend: Option<String>,
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
    let settings = match cmdline_args.settings {
        Some(settings) => {
           SettingsReader::read_config_file::<Settings>(&settings).unwrap() 
        },
        None => {
            load_settings_from_s3::<Settings>(S3_STORED_SETTINGS).await
        }
    };
    
    let frontend_path = match cmdline_args.frontend {
        Some(settings) => settings,
        None => "frontend".to_string(),
    };

    let cancel_token = CancellationToken::new();
    if let Err(_e) = aws_logging::init_cloudwatch_logger(&settings.logging) {
        tracing_subscriber::fmt::init();
    }

    let version = env!("CARGO_PKG_VERSION");

    info!("___/********Data Viewer v{}********\\___", version);

    info!(
        "Settings: {}",
        &to_string(&settings).expect("Failed to parse settings to json")
    );

    let db = db_client::startup_db(&settings.database).await;

    let state = Arc::new(AppState { db });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(service::health::health))
        .route("/symbols", get(service::symbols::symbols))
        .route("/strategy/{symbol}", get(service::strategy::strategy))
        .route("/universe", get(service::universe::universe))
        .route("/performance", get(service::performance::performance))
        .route("/metrics", get(service::metrics::metrics))
        .route("/watermarks", get(service::watermarks::watermarks))
        .with_state(state)
        .layer(cors)
        .fallback_service(ServeDir::new(frontend_path).append_index_html_on_directories(true));

    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
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
