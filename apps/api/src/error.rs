use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use droply_domain::DroplyError;
use serde_json::json;

/// Maps `DroplyError` (business failures, doc §37) onto HTTP responses.
/// Lives here rather than on `DroplyError` itself so `droply-domain` stays
/// free of any HTTP-framework dependency.
pub struct ApiError(pub DroplyError);

impl From<DroplyError> for ApiError {
    fn from(err: DroplyError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self.0 {
            DroplyError::InvalidUrl { .. } => (StatusCode::BAD_REQUEST, "invalid_url"),
            DroplyError::UnsupportedSource => {
                (StatusCode::UNPROCESSABLE_ENTITY, "unsupported_source")
            }
            DroplyError::ProtectedContent => (StatusCode::FORBIDDEN, "protected_content"),
            DroplyError::SourceUnavailable => (StatusCode::BAD_GATEWAY, "source_unavailable"),
            DroplyError::InsufficientStorage => {
                (StatusCode::INSUFFICIENT_STORAGE, "insufficient_storage")
            }
            DroplyError::DownloadCancelled => (StatusCode::CONFLICT, "download_cancelled"),
            DroplyError::ProcessingFailed { .. } => {
                (StatusCode::UNPROCESSABLE_ENTITY, "processing_failed")
            }
            DroplyError::InvalidStatusTransition { .. } => {
                (StatusCode::CONFLICT, "invalid_status_transition")
            }
        };

        (
            status,
            Json(json!({ "error": code, "message": self.0.to_string() })),
        )
            .into_response()
    }
}
