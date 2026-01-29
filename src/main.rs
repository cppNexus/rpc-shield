mod config;
mod identity;
mod proxy;
mod rate_limiter;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::proxy::{health_check, proxy_handler, ProxyState};
use crate::rate_limiter::RateLimiter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Путь к конфигурационному файлу
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    /// Режим работы: self-hosted или saas
    #[arg(short, long)]
    mode: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализация логирования
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,polymorph_proxy=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    // Загрузка конфигурации
    let config = Config::from_file(&args.config)?;
    tracing::info!("Configuration loaded from {}", args.config);
    tracing::info!("Operation mode: {:?}", config.server.mode);

    // Инициализация rate limiter
    let rate_limiter = Arc::new(RateLimiter::new(config.rate_limits.clone()));
    tracing::info!("Rate limiter initialized");

    // HTTP клиент для проксирования
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            config.rpc_backend.timeout_seconds,
        ))
        .build()?;

    // Создание общего состояния
    let state = Arc::new(ProxyState {
        rate_limiter,
        rpc_backend_url: config.rpc_backend.url.clone(),
        http_client,
    });

    // Создание маршрутов
    let app = Router::new()
        .route("/", post(proxy_handler))
        .route("/health", get(health_check))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Запуск сервера
    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("rpc-shield

 starting on {}", addr);
    tracing::info!("Backend RPC: {}", config.rpc_backend.url);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
