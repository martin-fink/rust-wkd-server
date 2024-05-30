use std::path::Path;
use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use axum::Router;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::config::Config;
use crate::keys::KeyDb;

pub mod errors;
pub mod keys;
pub mod policy;

#[derive(Clone)]
pub struct ApiContext {
    config: Arc<Config>,
    key_db: Arc<KeyDb>,
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let socket_addr: SocketAddr = format!("{}:{}", config.address, config.port)
        .as_str()
        .parse()?;
    let cache = KeyDb::new(Path::new(&config.keys_path)).await?;
    let app = api_router()
        .with_state(ApiContext {
            config: Arc::new(config),
            key_db: Arc::new(cache),
        })
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    info!("WKD server listening on {}", socket_addr);
    let listener = tokio::net::TcpListener::bind(socket_addr).await?;
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("error running HTTP server")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn api_router() -> Router<ApiContext> {
    keys::router().merge(policy::router())
}
