pub mod errors;
pub mod keys;
pub mod policy;

use crate::config::Config;
use anyhow::Context;
use axum::Router;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct ApiContext {
    config: Arc<Config>,
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let app = api_router()
        .with_state(ApiContext {
            config: Arc::new(config),
        })
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    axum::Server::bind(&"0.0.0.0:8080".parse()?)
        .serve(app.into_make_service())
        .await
        .context("error running HTTP server")
}

fn api_router() -> Router<ApiContext> {
    keys::router().merge(policy::router())
}
