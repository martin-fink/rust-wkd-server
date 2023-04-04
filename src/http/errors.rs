use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::error::Error;

pub enum ApiError {
    NotFound,
    Internal(String),
}

impl From<std::io::Error> for ApiError {
    fn from(value: std::io::Error) -> Self {
        Self::Internal(value.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value.to_string())
    }
}

impl From<Box<dyn Error>> for ApiError {
    fn from(value: Box<dyn Error>) -> Self {
        Self::Internal(value.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not Found.").into_response(),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
        }
    }
}
