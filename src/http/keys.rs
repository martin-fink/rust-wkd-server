use crate::http::errors::ApiError;
use crate::http::ApiContext;
use crate::utils::keys;
use axum::extract::{Host, Path, State};
use axum::routing::get;
use axum::Router;
use tracing::info;

type KeyResponse = Vec<u8>;

pub async fn get_key(
    State(state): State<ApiContext>,
    Path(hash): Path<String>,
    Host(domain): Host,
) -> Result<KeyResponse, ApiError> {
    if let Some(key) = keys::get_key_for_hash(&state.config.keys_path, &hash, &domain).await? {
        info!("Serving key for domain {domain}, hash {hash}.");
        Ok(key)
    } else {
        info!("No match found for domain {domain}, hash {hash}.");
        Err(ApiError::NotFound)
    }
}

pub fn router() -> Router<ApiContext> {
    Router::new().route("/.well-known/openpgpkey/hu/:key", get(get_key))
}
