use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DroplyError;

/// Lifecycle of a `Download`, per `docs/architecture.md` §8/§34.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadStatus {
    Pending,
    Analyzing,
    Ready,
    Queued,
    Downloading,
    Processing,
    Completed,
    Paused,
    Cancelled,
    Failed,
}

impl DownloadStatus {
    /// Whether this status is a terminal state (no further transitions).
    pub fn is_terminal(self) -> bool {
        matches!(self, DownloadStatus::Completed | DownloadStatus::Cancelled)
    }

    fn allowed_next(self) -> &'static [DownloadStatus] {
        use DownloadStatus::*;
        match self {
            Pending => &[Analyzing, Cancelled],
            Analyzing => &[Ready, Failed, Cancelled],
            Ready => &[Queued, Cancelled],
            Queued => &[Downloading, Cancelled],
            Downloading => &[Processing, Paused, Cancelled, Failed],
            Processing => &[Completed, Failed, Cancelled],
            Paused => &[Downloading, Cancelled],
            Failed => &[Queued, Cancelled],
            Completed => &[],
            Cancelled => &[],
        }
    }

    /// Validate and perform a transition, per the state machine above.
    ///
    /// Returns `DroplyError::InvalidStatusTransition` rather than panicking —
    /// callers (API handlers, job processor) always receive an untrusted
    /// desired next state and must handle rejection as a normal outcome.
    pub fn transition(self, next: DownloadStatus) -> Result<DownloadStatus, DroplyError> {
        if self.allowed_next().contains(&next) {
            Ok(next)
        } else {
            Err(DroplyError::InvalidStatusTransition {
                from: self,
                to: next,
            })
        }
    }

    /// Stable lowercase string form — used for persistence (a Postgres TEXT
    /// column, not a native enum, so adding a status never needs a
    /// migration) rather than the `Debug`/derived-JSON representations,
    /// which are an implementation detail that could change.
    pub fn as_str(self) -> &'static str {
        match self {
            DownloadStatus::Pending => "pending",
            DownloadStatus::Analyzing => "analyzing",
            DownloadStatus::Ready => "ready",
            DownloadStatus::Queued => "queued",
            DownloadStatus::Downloading => "downloading",
            DownloadStatus::Processing => "processing",
            DownloadStatus::Completed => "completed",
            DownloadStatus::Paused => "paused",
            DownloadStatus::Cancelled => "cancelled",
            DownloadStatus::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DroplyError> {
        match value {
            "pending" => Ok(DownloadStatus::Pending),
            "analyzing" => Ok(DownloadStatus::Analyzing),
            "ready" => Ok(DownloadStatus::Ready),
            "queued" => Ok(DownloadStatus::Queued),
            "downloading" => Ok(DownloadStatus::Downloading),
            "processing" => Ok(DownloadStatus::Processing),
            "completed" => Ok(DownloadStatus::Completed),
            "paused" => Ok(DownloadStatus::Paused),
            "cancelled" => Ok(DownloadStatus::Cancelled),
            "failed" => Ok(DownloadStatus::Failed),
            other => Err(DroplyError::ProcessingFailed {
                reason: format!("unknown download status: {other}"),
            }),
        }
    }
}

/// A user's download, per doc §8. The binary file itself lives on disk (or
/// the user's device, for direct-to-device downloads); this is the tracked
/// metadata row — status, progress, history.
#[derive(Debug, Clone, PartialEq)]
pub struct Download {
    pub id: Uuid,
    pub source_url: String,
    pub file_name: String,
    pub media_type: Option<String>,
    pub status: DownloadStatus,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl Download {
    pub fn new(
        source_url: String,
        file_name: String,
        media_type: Option<String>,
        total_bytes: Option<u64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_url,
            file_name,
            media_type,
            status: DownloadStatus::Pending,
            bytes_downloaded: 0,
            total_bytes,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    /// Move to `next`, per `DownloadStatus::transition`'s rules, and keep
    /// `started_at`/`completed_at` consistent with the new status. Leaves
    /// `self` unchanged on rejection — callers can retry with a different
    /// target status without having to reconstruct the entity.
    pub fn transition(&mut self, next: DownloadStatus) -> Result<(), DroplyError> {
        let next = self.status.transition(next)?;

        if next == DownloadStatus::Downloading && self.started_at.is_none() {
            self.started_at = Some(Utc::now());
        }
        if next.is_terminal() {
            self.completed_at = Some(Utc::now());
        }

        self.status = next;
        Ok(())
    }

    pub fn record_progress(&mut self, bytes_downloaded: u64) {
        self.bytes_downloaded = bytes_downloaded;
    }

    /// Convenience for the common "something went wrong" path: transitions
    /// to `Failed` and records why in one call.
    pub fn fail(&mut self, reason: impl Into<String>) -> Result<(), DroplyError> {
        self.transition(DownloadStatus::Failed)?;
        self.error = Some(reason.into());
        Ok(())
    }

    /// Doc §34: `Failed -> Retry -> Queued`. There's no partial-resume
    /// support yet (that's doc §6's "pause/resume where technically
    /// supported", Phase 6) — a retry always restarts from zero, so the
    /// stale error and byte count from the failed attempt are cleared.
    pub fn retry(&mut self) -> Result<(), DroplyError> {
        self.transition(DownloadStatus::Queued)?;
        self.bytes_downloaded = 0;
        self.error = None;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::DownloadStatus::*;

    #[test]
    fn happy_path_reaches_completed() {
        let s = Pending
            .transition(Analyzing)
            .unwrap()
            .transition(Ready)
            .unwrap()
            .transition(Queued)
            .unwrap()
            .transition(Downloading)
            .unwrap()
            .transition(Processing)
            .unwrap()
            .transition(Completed)
            .unwrap();
        assert_eq!(s, Completed);
        assert!(s.is_terminal());
    }

    #[test]
    fn failed_download_can_be_retried_via_queued() {
        let s = Downloading.transition(Failed).unwrap();
        assert_eq!(s, Failed);
        let retried = s.transition(Queued).unwrap();
        assert_eq!(retried, Queued);
    }

    #[test]
    fn paused_download_can_resume() {
        let s = Downloading
            .transition(Paused)
            .unwrap()
            .transition(Downloading);
        assert_eq!(s, Ok(Downloading));
    }

    #[test]
    fn cannot_skip_from_pending_to_downloading() {
        let result = Pending.transition(Downloading);
        assert!(result.is_err());
    }

    #[test]
    fn terminal_states_reject_every_transition() {
        for next in [
            Pending,
            Analyzing,
            Ready,
            Queued,
            Downloading,
            Processing,
            Completed,
            Paused,
            Cancelled,
            Failed,
        ] {
            assert!(Completed.transition(next).is_err());
            assert!(Cancelled.transition(next).is_err());
        }
    }

    #[test]
    fn cancellation_is_reachable_from_every_non_terminal_state() {
        for state in [
            Pending,
            Analyzing,
            Ready,
            Queued,
            Downloading,
            Processing,
            Paused,
            Failed,
        ] {
            assert!(
                state.transition(Cancelled).is_ok(),
                "{state:?} -> Cancelled should be allowed"
            );
        }
    }

    #[test]
    fn status_as_str_and_parse_round_trip_for_every_variant() {
        for status in [
            Pending,
            Analyzing,
            Ready,
            Queued,
            Downloading,
            Processing,
            Completed,
            Paused,
            Cancelled,
            Failed,
        ] {
            assert_eq!(super::DownloadStatus::parse(status.as_str()), Ok(status));
        }
    }

    #[test]
    fn parse_rejects_an_unknown_status() {
        assert!(super::DownloadStatus::parse("not-a-real-status").is_err());
    }

    mod download_entity {
        use super::super::{Download, DownloadStatus};

        fn sample() -> Download {
            Download::new(
                "https://example.com/movie.mp4".to_string(),
                "movie.mp4".to_string(),
                Some("video/mp4".to_string()),
                Some(1024),
            )
        }

        #[test]
        fn new_download_starts_pending_with_no_timestamps_set() {
            let download = sample();
            assert_eq!(download.status, DownloadStatus::Pending);
            assert_eq!(download.bytes_downloaded, 0);
            assert!(download.started_at.is_none());
            assert!(download.completed_at.is_none());
            assert!(download.error.is_none());
        }

        #[test]
        fn transitioning_to_downloading_sets_started_at_once() {
            let mut download = sample();
            download.transition(DownloadStatus::Analyzing).unwrap();
            download.transition(DownloadStatus::Ready).unwrap();
            download.transition(DownloadStatus::Queued).unwrap();
            download.transition(DownloadStatus::Downloading).unwrap();
            let first_started_at = download.started_at;
            assert!(first_started_at.is_some());

            download.transition(DownloadStatus::Paused).unwrap();
            download.transition(DownloadStatus::Downloading).unwrap();

            assert_eq!(
                download.started_at, first_started_at,
                "resuming must not overwrite the original started_at"
            );
        }

        #[test]
        fn reaching_a_terminal_status_sets_completed_at() {
            let mut download = sample();
            download.transition(DownloadStatus::Cancelled).unwrap();
            assert!(download.completed_at.is_some());
        }

        #[test]
        fn a_rejected_transition_leaves_the_download_unchanged() {
            let mut download = sample();
            let before = download.clone();

            let result = download.transition(DownloadStatus::Downloading);

            assert!(result.is_err());
            assert_eq!(download, before);
        }

        #[test]
        fn fail_transitions_to_failed_and_records_the_reason() {
            let mut download = sample();
            download.transition(DownloadStatus::Analyzing).unwrap();
            download.transition(DownloadStatus::Ready).unwrap();
            download.transition(DownloadStatus::Queued).unwrap();
            download.transition(DownloadStatus::Downloading).unwrap();

            download.fail("connection reset").unwrap();

            assert_eq!(download.status, DownloadStatus::Failed);
            assert_eq!(download.error.as_deref(), Some("connection reset"));
        }

        #[test]
        fn record_progress_updates_bytes_downloaded() {
            let mut download = sample();
            download.record_progress(512);
            assert_eq!(download.bytes_downloaded, 512);
        }

        #[test]
        fn retry_moves_a_failed_download_to_queued_and_clears_error_and_progress() {
            let mut download = sample();
            download.transition(DownloadStatus::Analyzing).unwrap();
            download.transition(DownloadStatus::Ready).unwrap();
            download.transition(DownloadStatus::Queued).unwrap();
            download.transition(DownloadStatus::Downloading).unwrap();
            download.record_progress(999);
            download.fail("connection reset").unwrap();

            download.retry().unwrap();

            assert_eq!(download.status, DownloadStatus::Queued);
            assert_eq!(download.bytes_downloaded, 0);
            assert!(download.error.is_none());
        }

        #[test]
        fn retry_is_rejected_when_not_currently_failed() {
            let mut download = sample();
            assert!(download.retry().is_err());
        }
    }
}
