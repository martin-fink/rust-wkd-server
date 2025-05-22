use crate::http::errors::ApiError;
use crate::http::ApiContext;
use crate::policy::get_policy;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::Router;
use axum_extra::extract::Host;

type PolicyResponse = String;

const EMPTY_POLICY: &str = "# Empty policy\n";

fn get_policy_for_domain(state: &ApiContext, domain: &str) -> Result<PolicyResponse, ApiError> {
    match &state.config.policy {
        None => Ok(EMPTY_POLICY.into()),
        Some(path) => Ok(get_policy(path, domain)
            .map_err(|_| ApiError::Internal("".into()))?
            .unwrap_or_default()),
    }
}

pub async fn get_policy_direct(
    State(state): State<ApiContext>,
    Host(domain): Host,
) -> Result<PolicyResponse, ApiError> {
    get_policy_for_domain(&state, &domain)
}

pub async fn get_policy_advanced(
    State(state): State<ApiContext>,
    Path(domain): Path<String>,
) -> Result<PolicyResponse, ApiError> {
    get_policy_for_domain(&state, &domain)
}

pub fn router() -> Router<ApiContext> {
    Router::new()
        .route("/.well-known/openpgpkey/policy", get(get_policy_direct))
        .route(
            "/.well-known/openpgpkey/{domain}/policy",
            get(get_policy_advanced),
        )
}
