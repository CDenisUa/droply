use thiserror::Error;

use crate::download::DownloadStatus;

/// Typed business failures, per `docs/architecture.md` §37. These are
/// expected outcomes (bad user input, unsupported sources, business rule
/// violations) and should be matched on and mapped to API responses, not
/// treated as exceptional. Unexpected technical failures (I/O, DB) stay as
/// `anyhow::Error` at the infra/api boundary instead of joining this enum.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DroplyError {
    #[error("unsupported source")]
    UnsupportedSource,

    #[error("invalid URL: {reason}")]
    InvalidUrl { reason: String },

    #[error("content requires DRM circumvention, which Droply will not perform")]
    ProtectedContent,

    #[error("source is unavailable")]
    SourceUnavailable,

    #[error("insufficient storage")]
    InsufficientStorage,

    #[error("download was cancelled")]
    DownloadCancelled,

    #[error("media processing failed: {reason}")]
    ProcessingFailed { reason: String },

    #[error("cannot transition download from {from:?} to {to:?}")]
    InvalidStatusTransition {
        from: DownloadStatus,
        to: DownloadStatus,
    },
}
