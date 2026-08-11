use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use droply_application::{DownloadRepository, DownloadStrategyResolver, MediaSourceResolver};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Cancellation tokens for downloads currently being executed by
/// `download_runner`, keyed by `Download::id`. A download not present here
/// is either not yet started, or already finished (there is no lingering
/// entry to clean up incorrectly cancel-able — `download_runner` removes
/// its own entry when the task ends).
pub type ActiveCancellations = Arc<Mutex<HashMap<Uuid, CancellationToken>>>;

/// Shared application state handed to route handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub source_resolver: Arc<MediaSourceResolver>,
    pub download_repository: Arc<dyn DownloadRepository>,
    pub download_strategy_resolver: Arc<DownloadStrategyResolver>,
    pub active_cancellations: ActiveCancellations,
    /// Where in-progress/completed downloads are staged before being served
    /// through `/api/downloads/{id}/content`. Not permanent storage — see
    /// doc §44 ("never persist media permanently on free backend storage").
    pub temp_storage_path: PathBuf,
}

impl AppState {
    pub fn download_file_path(&self, id: Uuid) -> PathBuf {
        self.temp_storage_path.join(id.to_string())
    }
}
