//! Use-case orchestration and trait boundaries (`MediaSourceAnalyzer`,
//! `DownloadStrategy`, `MediaProcessor`, `JobQueue`, `UrlValidator`, ...).
//!
//! Populated incrementally starting Phase 1 (direct file downloader), see
//! `docs/CURRENT_STATE.md` and AGENTS.md §15 ("don't create a trait until
//! there's a concrete near-term use for it").

pub mod url_validator;

pub use url_validator::UrlValidator;
