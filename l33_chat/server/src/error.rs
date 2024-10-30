use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;
use tracing::error;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Other(#[from] anyhow::Error),

    #[error("{0} not found")]
    NotFound(&'static str),

    #[error(transparent)]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("User is not a participant of the room")]
    Forbidden,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        use Error::*;

        error!(error = ?self);

        match self {
            Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            Other(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Something went wrong"),
            ),
            ValidationError(_) => (
                StatusCode::BAD_REQUEST,
                format!("Input validation error: [{}]", self).replace('\n', ", "),
            ),
            NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
        }
        .into_response()
    }
}
