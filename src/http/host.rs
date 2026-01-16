use crate::http::errors::ApiError;
use axum::http::{HeaderMap, HeaderValue, uri::Authority};
use axum_extra::headers::{HeaderMapExt, Host};

const X_FORWARDED_HOST: &str = "X-Forwarded-Host";

fn parse_forwarded_host(value: &HeaderValue) -> Result<Host, ()> {
    let authority = Authority::try_from(value.as_bytes()).map_err(|_| ())?;
    Ok(Host::from(authority))
}

pub fn domain_from_headers(headers: &HeaderMap) -> Result<String, ApiError> {
    match headers.get(X_FORWARDED_HOST).map(parse_forwarded_host) {
        Some(Ok(forwarded_host)) => {
            return Ok(forwarded_host.hostname().to_string());
        }
        Some(Err(_)) => {
            return Err(ApiError::BadRequest(
                "Invalid X-Forwarded-Host header.".into(),
            ));
        }
        None => {}
    }

    match headers.typed_try_get::<Host>() {
        Ok(Some(host)) => Ok(host.hostname().to_string()),
        _ => Err(ApiError::BadRequest(
            "Invalid or missing Host header.".into(),
        )),
    }
}
