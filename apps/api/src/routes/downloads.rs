use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use droply_domain::{Download, DownloadStatus, DroplyError};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::download_runner;
use crate::error::ApiError;
use crate::state::AppState;

mod content;

#[derive(Debug, Deserialize)]
pub struct CreateDownloadRequest {
    url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResponse {
    id: Uuid,
    source_url: String,
    file_name: String,
    media_type: Option<String>,
    status: DownloadStatus,
    bytes_downloaded: u64,
    total_bytes: Option<u64>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    error: Option<String>,
}

impl From<&Download> for DownloadResponse {
    fn from(d: &Download) -> Self {
        Self {
            id: d.id,
            source_url: d.source_url.clone(),
            file_name: d.file_name.clone(),
            media_type: d.media_type.clone(),
            status: d.status,
            bytes_downloaded: d.bytes_downloaded,
            total_bytes: d.total_bytes,
            created_at: d.created_at,
            started_at: d.started_at,
            completed_at: d.completed_at,
            error: d.error.clone(),
        }
    }
}

/// `POST /api/downloads` — doc §26. Takes the source URL directly rather
/// than doc's original `{ sourceId, variantId }` shape: that shape assumes
/// a previously-analyzed `MediaSource` persisted server-side with an ID a
/// client can reference, which doesn't exist for direct files (a direct
/// file has exactly one variant — itself) and isn't built until variant
/// selection is real (HLS/DASH, Phase 4/5). Re-analyzing the URL here is
/// deliberate — see `docs/DECISIONS.md` ADR 0006.
async fn create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateDownloadRequest>,
) -> Result<(StatusCode, Json<DownloadResponse>), ApiError> {
    let url = Url::parse(&payload.url).map_err(|err| DroplyError::InvalidUrl {
        reason: err.to_string(),
    })?;

    let source = state.source_resolver.resolve(&url).await?;

    let download = Download::new(
        payload.url,
        source.title,
        source.mime_type,
        source.size_bytes,
    );
    persist_new(&state, &download).await?;

    download_runner::spawn(state.clone(), download.clone(), source.source_type);

    Ok((StatusCode::ACCEPTED, Json((&download).into())))
}

async fn persist_new(state: &AppState, download: &Download) -> Result<(), ApiError> {
    state
        .download_repository
        .create(download)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to persist new download");
            ApiError(DroplyError::ProcessingFailed {
                reason: "could not save download".to_string(),
            })
        })
}

async fn find_or_404(state: &AppState, id: Uuid) -> Result<Download, ApiError> {
    state
        .download_repository
        .find_by_id(id)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to look up download");
            ApiError(DroplyError::ProcessingFailed {
                reason: "could not look up download".to_string(),
            })
        })?
        .ok_or(ApiError(DroplyError::NotFound {
            resource: format!("download {id}"),
        }))
}

const DEFAULT_LIST_LIMIT: i64 = 50;
const MAX_LIST_LIMIT: i64 = 200;

#[derive(Debug, Deserialize)]
pub struct ListDownloadsQuery {
    limit: Option<i64>,
}

/// `GET /api/downloads` — most recent downloads first, backs the doc's
/// "History"/"Downloads" view (§2). Not in the original doc's §26 API
/// list (which only names single-resource endpoints) but needed for the
/// frontend to show more than the one download it just created.
async fn list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListDownloadsQuery>,
) -> Result<Json<Vec<DownloadResponse>>, ApiError> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_LIST_LIMIT)
        .clamp(1, MAX_LIST_LIMIT);

    let downloads = state
        .download_repository
        .list_recent(limit)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to list downloads");
            ApiError(DroplyError::ProcessingFailed {
                reason: "could not list downloads".to_string(),
            })
        })?;

    Ok(Json(downloads.iter().map(DownloadResponse::from).collect()))
}

/// `GET /api/downloads/{id}` — current status/progress.
async fn status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<DownloadResponse>, ApiError> {
    let download = find_or_404(&state, id).await?;
    Ok(Json((&download).into()))
}

/// `POST /api/downloads/{id}/cancel` — signals the running task (if any)
/// and returns immediately; the task observes the signal and persists
/// `Cancelled` itself, so the response reflects the state *before* that
/// happens (still `Downloading`, typically), not after.
async fn cancel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<DownloadResponse>, ApiError> {
    let download = find_or_404(&state, id).await?;

    #[allow(clippy::unwrap_used)]
    if let Some(token) = state.active_cancellations.lock().unwrap().get(&id) {
        token.cancel();
    }

    Ok(Json((&download).into()))
}

/// `POST /api/downloads/{id}/retry` — doc §34 ("Failed -> Retry ->
/// Queued"). Only valid from `Failed`; re-resolves the source to determine
/// which `DownloadStrategy` to use again (the `Download` row doesn't store
/// `SourceType` — see doc §8's field list — so this is re-derived rather
/// than added as a new column purely to save one HTTP call).
async fn retry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DownloadResponse>), ApiError> {
    let mut download = find_or_404(&state, id).await?;

    if download.status != DownloadStatus::Failed {
        return Err(ApiError(DroplyError::InvalidStatusTransition {
            from: download.status,
            to: DownloadStatus::Queued,
        }));
    }

    let url = Url::parse(&download.source_url).map_err(|err| DroplyError::InvalidUrl {
        reason: err.to_string(),
    })?;
    let source = state.source_resolver.resolve(&url).await?;

    download.retry()?;
    persist_new_or_update(&state, &download).await?;

    download_runner::spawn(state.clone(), download.clone(), source.source_type);

    Ok((StatusCode::ACCEPTED, Json((&download).into())))
}

async fn persist_new_or_update(state: &AppState, download: &Download) -> Result<(), ApiError> {
    state
        .download_repository
        .update(download)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to persist retried download");
            ApiError(DroplyError::ProcessingFailed {
                reason: "could not save download".to_string(),
            })
        })
}

/// `GET /api/downloads/{id}/content` — doc §26/§28. Only serves the file
/// once `status == Completed`; there is no partial/in-progress streaming
/// in this phase (see `docs/CURRENT_STATE.md`).
async fn content(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let download = find_or_404(&state, id).await?;

    if download.status != DownloadStatus::Completed {
        return Err(ApiError(DroplyError::ProcessingFailed {
            reason: format!(
                "download is not ready yet (status: {})",
                download.status.as_str()
            ),
        }));
    }

    let path = state.download_file_path(id);
    let range_header = headers
        .get(axum::http::header::RANGE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    content::serve_file(&path, &download, range_header.as_deref())
        .await
        .map_err(ApiError)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/downloads", post(create).get(list))
        .route("/api/downloads/:id", get(status))
        .route("/api/downloads/:id/cancel", post(cancel))
        .route("/api/downloads/:id/retry", post(retry))
        .route("/api/downloads/:id/content", get(content))
}
