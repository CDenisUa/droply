use serde::{Deserialize, Serialize};

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
}
