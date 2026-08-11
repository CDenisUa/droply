use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use droply_domain::{Download, DownloadStatus, DroplyError, SourceType};
use tokio_util::sync::CancellationToken;
use url::Url;
use uuid::Uuid;

use crate::state::AppState;

/// How often in-progress byte counts are persisted. Deliberately not on
/// every chunk — `DownloadStrategy::execute` updates its progress atomic
/// far more often than that, and flushing every write to Postgres would
/// turn a large download into a large number of tiny UPDATE statements for
/// no user-visible benefit.
const PROGRESS_FLUSH_INTERVAL: Duration = Duration::from_millis(500);

/// Starts executing `download` in the background and returns immediately.
/// The caller (an HTTP handler) has already persisted `download` in its
/// initial state (`Pending`, for a fresh download; `Failed` about to become
/// `Queued`, for a retry) — this function owns every state transition from
/// here on.
pub fn spawn(state: Arc<AppState>, download: Download, source_type: SourceType) {
    let cancellation = CancellationToken::new();
    #[allow(clippy::unwrap_used)]
    state
        .active_cancellations
        .lock()
        .unwrap()
        .insert(download.id, cancellation.clone());

    tokio::spawn(async move {
        run(&state, download, source_type, cancellation).await;
    });
}

async fn run(
    state: &Arc<AppState>,
    mut download: Download,
    source_type: SourceType,
    cancellation: CancellationToken,
) {
    let id = download.id;
    run_inner(state, &mut download, source_type, cancellation).await;

    #[allow(clippy::unwrap_used)]
    state.active_cancellations.lock().unwrap().remove(&id);
}

/// Walks `download` forward to `Downloading`, from wherever it currently
/// is: `Pending` for a fresh download (goes through the full
/// Analyzing/Ready/Queued sequence), `Queued` for a retry (the caller
/// already moved it there from `Failed` — doc §34's "Failed -> Retry ->
/// Queued" rejoins the main flow, it doesn't redo analysis).
fn advance_to_downloading(download: &mut Download) -> Result<(), DroplyError> {
    loop {
        match download.status {
            DownloadStatus::Downloading => return Ok(()),
            DownloadStatus::Pending => download.transition(DownloadStatus::Analyzing)?,
            DownloadStatus::Analyzing => download.transition(DownloadStatus::Ready)?,
            DownloadStatus::Ready => download.transition(DownloadStatus::Queued)?,
            DownloadStatus::Queued => download.transition(DownloadStatus::Downloading)?,
            other => {
                return Err(DroplyError::InvalidStatusTransition {
                    from: other,
                    to: DownloadStatus::Downloading,
                })
            }
        }
    }
}

async fn run_inner(
    state: &AppState,
    download: &mut Download,
    source_type: SourceType,
    cancellation: CancellationToken,
) {
    let Ok(strategy) = state.download_strategy_resolver.resolve(source_type) else {
        let _ = download.fail("unsupported source");
        persist(state, download).await;
        return;
    };

    if advance_to_downloading(download).is_err() {
        // Already in a terminal state (e.g. cancelled before the task got
        // scheduled) — nothing left to do.
        persist(state, download).await;
        return;
    }
    persist(state, download).await;

    let Ok(source_url) = Url::parse(&download.source_url) else {
        let _ = download.fail("stored source_url is no longer a valid URL");
        persist(state, download).await;
        return;
    };

    let destination = state.download_file_path(download.id);
    let progress = Arc::new(AtomicU64::new(0));

    let result = run_with_progress_flushes(state, download.id, &progress, || {
        strategy.execute(
            &source_url,
            &destination,
            progress.clone(),
            cancellation.clone(),
        )
    })
    .await;

    download.record_progress(progress.load(Ordering::Relaxed));

    match result {
        Ok(()) => {
            // No real processing step for a direct file (that's the doc's
            // hook for future remux/convert work, Phase 4+) — pass through.
            let _ = download.transition(DownloadStatus::Processing);
            let _ = download.transition(DownloadStatus::Completed);
        }
        Err(DroplyError::DownloadCancelled) => {
            let _ = download.transition(DownloadStatus::Cancelled);
        }
        Err(err) => {
            let _ = download.fail(err.to_string());
        }
    }

    persist(state, download).await;
}

/// Runs `make_future()` while concurrently flushing `progress` to Postgres
/// on an interval, until `make_future()`'s future resolves.
async fn run_with_progress_flushes<Fut>(
    state: &AppState,
    download_id: Uuid,
    progress: &Arc<AtomicU64>,
    make_future: impl FnOnce() -> Fut,
) -> Result<(), DroplyError>
where
    Fut: std::future::Future<Output = Result<(), DroplyError>>,
{
    let flush_loop = async {
        loop {
            tokio::time::sleep(PROGRESS_FLUSH_INTERVAL).await;
            if let Ok(Some(mut current)) = state.download_repository.find_by_id(download_id).await {
                current.record_progress(progress.load(Ordering::Relaxed));
                let _ = state.download_repository.update(&current).await;
            }
        }
    };

    tokio::select! {
        result = make_future() => result,
        () = flush_loop => unreachable!("flush_loop never completes on its own"),
    }
}

async fn persist(state: &AppState, download: &Download) {
    if let Err(err) = state.download_repository.update(download).await {
        tracing::error!(download_id = %download.id, error = %err, "failed to persist download state");
    }
}
