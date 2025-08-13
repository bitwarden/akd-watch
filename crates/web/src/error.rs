use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ApiError {
    #[error("Not found")]
    NotFound,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error")]
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::BadRequest(e) => (StatusCode::BAD_REQUEST, e),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, msg).into_response()
    }
}
