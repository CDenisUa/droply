//! Pure domain types for Droply: no I/O, no async runtime, no framework
//! dependencies. See `docs/architecture.md` for the layering rules.

pub mod download;
pub mod error;
pub mod media_source;

pub use download::{Download, DownloadStatus};
pub use error::DroplyError;
pub use media_source::{derive_filename, MediaSourceResult, SourceType};
