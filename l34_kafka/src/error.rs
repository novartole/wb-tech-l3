use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;
use tracing::error;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("connection to postgres db failed: {0}")]
    PgConnFailed(#[from] bb8::RunError<tokio_postgres::Error>),

    #[error("postgres failed to execute query: {0}")]
    PgQueryFailed(#[from] tokio_postgres::Error),

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
            Error::ValidationError(_) => (
                StatusCode::BAD_REQUEST,
                format!("Input validation error: [{}]", self).replace('\n', ", "),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Something went wrong"),
            ),
        }
        .into_response()
    }
}
