use async_trait::async_trait;
use droply_domain::Download;
use uuid::Uuid;

/// Persists `Download` rows. Not a generic `Repository<T>` — these are the
/// specific operations the download use cases actually need (AGENTS.md
/// rule 16). Returns `anyhow::Result` rather than `DroplyError`: a failure
/// here is an infrastructure problem (DB unreachable, constraint violation),
/// not one of `DroplyError`'s expected business outcomes — see
/// `docs/architecture.md` §37.
#[async_trait]
pub trait DownloadRepository: Send + Sync {
    async fn create(&self, download: &Download) -> anyhow::Result<()>;
    async fn find_by_id(&self, id: Uuid) -> anyhow::Result<Option<Download>>;
    /// Persists the full current state of `download` (status, progress,
    /// timestamps, error) — callers mutate a `Download` via its domain
    /// methods (`transition`, `record_progress`, `fail`) and then call this
    /// to save the result, rather than the repository exposing separate
    /// per-field update methods.
    async fn update(&self, download: &Download) -> anyhow::Result<()>;
    /// Most recent downloads first — backs the doc's "History" view (§2).
    async fn list_recent(&self, limit: i64) -> anyhow::Result<Vec<Download>>;
}
