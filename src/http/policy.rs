use crate::http::errors::ApiError;
use crate::http::ApiContext;
use axum::extract::State;
use axum::routing::get;
use axum::Router;
use std::fs;

type PolicyResponse = String;

const EMPTY_POLICY: &str = "# Empty policy\n";

pub async fn get_policy(State(state): State<ApiContext>) -> Result<PolicyResponse, ApiError> {
    match &state.config.policy {
        None => Ok(EMPTY_POLICY.into()),
        Some(path) => fs::read_to_string(path).map_err(|_| ApiError::Internal("".into())),
    }
}

pub fn router() -> Router<ApiContext> {
    Router::new().route("/.well-known/openpgpkey/policy", get(get_policy))
}
