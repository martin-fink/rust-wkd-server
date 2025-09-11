use crate::http::ApiContext;
use crate::http::errors::ApiError;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum_extra::extract::Host;
use serde::Deserialize;
use tracing::info;

async fn get_key(
    state: &ApiContext,
    hash: &str,
    domain: &str,
    username: Option<&String>,
) -> Result<Vec<u8>, ApiError> {
    if let Some(key) = state.key_db.get(hash, domain, username).await? {
        info!("Serving key for domain {domain}, hash {hash}.");
        Ok(key)
    } else {
        info!("No match found for domain {domain}, hash {hash}.");
        Err(ApiError::NotFound)
    }
}

pub async fn get_key_direct(
    State(state): State<ApiContext>,
    Path(hash): Path<String>,
    Host(domain): Host,
) -> Result<Vec<u8>, ApiError> {
    get_key(&state, &hash, &domain, None).await
}

#[derive(Deserialize)]
pub struct UsernameParam {
    #[serde(rename = "l")]
    username: String,
}

pub async fn get_key_advanced(
    State(state): State<ApiContext>,
    Path((domain, hash)): Path<(String, String)>,
    Query(UsernameParam { username }): Query<UsernameParam>,
) -> Result<Vec<u8>, ApiError> {
    get_key(&state, &hash, &domain, Some(&username)).await
}

pub fn router() -> Router<ApiContext> {
    Router::new()
        .route("/.well-known/openpgpkey/hu/{key}", get(get_key_direct))
        .route(
            "/.well-known/openpgpkey/{domain}/hu/{key}",
            get(get_key_advanced),
        )
}
