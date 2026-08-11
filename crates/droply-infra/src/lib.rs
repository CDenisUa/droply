//! Concrete implementations of `droply-application` boundaries: Postgres,
//! outbound HTTP, filesystem. Only what Phase 0 needs (DB pool + readiness
//! ping) exists so far — see `docs/CURRENT_STATE.md`.

pub mod direct_file_analyzer;
pub mod direct_file_download_strategy;
pub mod download_repository;
pub(crate) mod http;
pub mod postgres;
pub mod url_validator;

pub use direct_file_analyzer::DirectFileAnalyzer;
pub use direct_file_download_strategy::DirectFileDownloadStrategy;
pub use download_repository::PostgresDownloadRepository;
pub use postgres::{create_pool, create_pool_with_timeout, ping};
pub use url_validator::SsrfSafeUrlValidator;
