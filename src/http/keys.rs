use crate::http::errors::ApiError;
use crate::http::ApiContext;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::Router;
use axum_extra::extract::Host;
use tracing::info;

pub async fn get_key(
    State(state): State<ApiContext>,
    Path(hash): Path<String>,
    Host(domain): Host,
) -> Result<Vec<u8>, ApiError> {
    if let Some(key) = state.key_db.get(&hash, &domain).await? {
        info!("Serving key for domain {domain}, hash {hash}.");
        Ok(key)
    } else {
        info!("No match found for domain {domain}, hash {hash}.");
        Err(ApiError::NotFound)
    }
}

pub fn router() -> Router<ApiContext> {
    Router::new().route("/.well-known/openpgpkey/hu/{key}", get(get_key))
}
