use std::sync::Arc;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use droply_domain::{DroplyError, MediaSourceResult, SourceType};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResponse {
    source_type: SourceType,
    title: String,
    mime_type: Option<String>,
    size_bytes: Option<u64>,
    duration_seconds: Option<f64>,
}

impl From<MediaSourceResult> for AnalyzeResponse {
    fn from(result: MediaSourceResult) -> Self {
        Self {
            source_type: result.source_type,
            title: result.title,
            mime_type: result.mime_type,
            size_bytes: result.size_bytes,
            duration_seconds: result.duration_seconds,
        }
    }
}

/// `POST /api/sources/analyze` — doc §26. URL validation happens inside
/// whichever analyzer ends up making the HTTP call (each analyzer owns its
/// own `UrlValidator` instance, per AGENTS.md rule 9); this handler only
/// rejects a URL that doesn't even parse.
async fn analyze(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, ApiError> {
    let url = Url::parse(&payload.url).map_err(|err| DroplyError::InvalidUrl {
        reason: err.to_string(),
    })?;

    let result = state.source_resolver.resolve(&url).await?;

    Ok(Json(result.into()))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/api/sources/analyze", post(analyze))
}
