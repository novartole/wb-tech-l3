use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("connection to redis repo failed: {0}")]
    RedisConnFailed(#[from] bb8::RunError<redis::RedisError>),

    #[error("redis failed to execute query: {0}")]
    RedisQueryFailed(#[from] redis::RedisError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),

    #[error("{0} not found")]
    NotFound(&'static str),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        error!(error = ?self);

        match self {
            Error::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Something went wrong"),
            ),
        }
        .into_response()
    }
}
