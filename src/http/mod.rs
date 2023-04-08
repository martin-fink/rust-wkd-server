pub mod errors;
pub mod keys;
pub mod policy;

use crate::config::Config;
use anyhow::Context;
use axum::Router;
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct ApiContext {
    config: Arc<Config>,
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let socket_addr: SocketAddr = format!("{}:{}", config.address, config.port)
        .as_str()
        .parse()?;
    let app = api_router()
        .with_state(ApiContext {
            config: Arc::new(config),
        })
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    axum::Server::bind(&socket_addr)
        .serve(app.into_make_service())
        .await
        .context("error running HTTP server")
}

fn api_router() -> Router<ApiContext> {
    keys::router().merge(policy::router())
}
