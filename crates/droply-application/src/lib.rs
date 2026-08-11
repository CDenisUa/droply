//! Use-case orchestration and trait boundaries (`MediaSourceAnalyzer`,
//! `DownloadStrategy`, `MediaProcessor`, `JobQueue`, `UrlValidator`, ...).
//!
//! Populated incrementally starting Phase 1 (direct file downloader), see
//! `docs/CURRENT_STATE.md` and AGENTS.md §15 ("don't create a trait until
//! there's a concrete near-term use for it").

pub mod download_repository;
pub mod download_strategy;
pub mod media_source_analyzer;
pub mod url_validator;

pub use download_repository::DownloadRepository;
pub use download_strategy::{DownloadStrategy, DownloadStrategyResolver};
pub use media_source_analyzer::{MediaSourceAnalyzer, MediaSourceResolver};
pub use url_validator::UrlValidator;
