use async_trait::async_trait;
use droply_application::DownloadRepository;
use droply_domain::{Download, DownloadStatus};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PostgresDownloadRepository {
    pool: PgPool,
}

impl PostgresDownloadRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Row shape as it actually comes back from Postgres — `bytes_downloaded`/
/// `total_bytes` are `BIGINT` (signed), so this stays `i64` here and is
/// converted to the domain's unsigned `u64` only when building a `Download`.
///
/// Uses the runtime-checked `FromRow` derive (`sqlx::query_as`), not the
/// `query_as!`/`query!` macros — those verify queries against a *live*
/// database at compile time, which would make `cargo build` itself require
/// a running, already-migrated Postgres. `docs/architecture.md` §3 rules out
/// that kind of build-time dependency.
#[derive(sqlx::FromRow)]
struct DownloadRow {
    id: Uuid,
    source_url: String,
    file_name: String,
    media_type: Option<String>,
    status: String,
    bytes_downloaded: i64,
    total_bytes: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    error: Option<String>,
}

impl TryFrom<DownloadRow> for Download {
    type Error = anyhow::Error;

    fn try_from(row: DownloadRow) -> Result<Self, Self::Error> {
        Ok(Download {
            id: row.id,
            source_url: row.source_url,
            file_name: row.file_name,
            media_type: row.media_type,
            status: DownloadStatus::parse(&row.status)?,
            bytes_downloaded: u64::try_from(row.bytes_downloaded)?,
            total_bytes: row.total_bytes.map(u64::try_from).transpose()?,
            created_at: row.created_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
            error: row.error,
        })
    }
}

const SELECT_COLUMNS: &str = "id, source_url, file_name, media_type, status, \
    bytes_downloaded, total_bytes, created_at, started_at, completed_at, error";

#[async_trait]
impl DownloadRepository for PostgresDownloadRepository {
    async fn create(&self, download: &Download) -> anyhow::Result<()> {
        let bytes_downloaded = i64::try_from(download.bytes_downloaded)?;
        let total_bytes = download.total_bytes.map(i64::try_from).transpose()?;

        sqlx::query(
            r#"
            INSERT INTO downloads
                (id, source_url, file_name, media_type, status,
                 bytes_downloaded, total_bytes, created_at, started_at,
                 completed_at, error)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(download.id)
        .bind(&download.source_url)
        .bind(&download.file_name)
        .bind(&download.media_type)
        .bind(download.status.as_str())
        .bind(bytes_downloaded)
        .bind(total_bytes)
        .bind(download.created_at)
        .bind(download.started_at)
        .bind(download.completed_at)
        .bind(&download.error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> anyhow::Result<Option<Download>> {
        let row: Option<DownloadRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLUMNS} FROM downloads WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Download::try_from).transpose()
    }

    async fn update(&self, download: &Download) -> anyhow::Result<()> {
        let bytes_downloaded = i64::try_from(download.bytes_downloaded)?;
        let total_bytes = download.total_bytes.map(i64::try_from).transpose()?;

        sqlx::query(
            r#"
            UPDATE downloads
            SET status = $2,
                bytes_downloaded = $3,
                total_bytes = $4,
                started_at = $5,
                completed_at = $6,
                error = $7
            WHERE id = $1
            "#,
        )
        .bind(download.id)
        .bind(download.status.as_str())
        .bind(bytes_downloaded)
        .bind(total_bytes)
        .bind(download.started_at)
        .bind(download.completed_at)
        .bind(&download.error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_recent(&self, limit: i64) -> anyhow::Result<Vec<Download>> {
        let rows: Vec<DownloadRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLUMNS} FROM downloads ORDER BY created_at DESC LIMIT $1"
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Download::try_from).collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use droply_domain::DownloadStatus;

    /// Requires a real, reachable Postgres with migrations applied — see
    /// `docs/CURRENT_STATE.md`. `cargo test -- --include-ignored`.
    async fn repository() -> PostgresDownloadRepository {
        let database_url =
            std::env::var("DATABASE_URL").expect("set DATABASE_URL to a reachable Postgres");
        let pool = crate::create_pool(&database_url).await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        PostgresDownloadRepository::new(pool)
    }

    fn sample_download() -> Download {
        Download::new(
            "https://example.com/movie.mp4".to_string(),
            "movie.mp4".to_string(),
            Some("video/mp4".to_string()),
            Some(123_456),
        )
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at a live Postgres"]
    async fn create_then_find_by_id_round_trips_the_download() {
        let repo = repository().await;
        let download = sample_download();

        repo.create(&download).await.unwrap();
        let found = repo.find_by_id(download.id).await.unwrap();

        assert_eq!(found, Some(download));
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at a live Postgres"]
    async fn find_by_id_returns_none_for_an_unknown_id() {
        let repo = repository().await;
        let found = repo.find_by_id(Uuid::new_v4()).await.unwrap();
        assert_eq!(found, None);
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at a live Postgres"]
    async fn update_persists_status_progress_and_error() {
        let repo = repository().await;
        let mut download = sample_download();
        repo.create(&download).await.unwrap();

        download.transition(DownloadStatus::Analyzing).unwrap();
        download.transition(DownloadStatus::Ready).unwrap();
        download.transition(DownloadStatus::Queued).unwrap();
        download.transition(DownloadStatus::Downloading).unwrap();
        download.record_progress(64_000);
        repo.update(&download).await.unwrap();

        let found = repo.find_by_id(download.id).await.unwrap().unwrap();
        assert_eq!(found.status, DownloadStatus::Downloading);
        assert_eq!(found.bytes_downloaded, 64_000);
        assert!(found.started_at.is_some());

        download.fail("connection reset").unwrap();
        repo.update(&download).await.unwrap();

        let found = repo.find_by_id(download.id).await.unwrap().unwrap();
        assert_eq!(found.status, DownloadStatus::Failed);
        assert_eq!(found.error.as_deref(), Some("connection reset"));
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at a live Postgres"]
    async fn list_recent_orders_newest_first_and_respects_the_limit() {
        let repo = repository().await;
        let mut ids_oldest_to_newest = Vec::new();
        for _ in 0..3 {
            let download = sample_download();
            ids_oldest_to_newest.push(download.id);
            repo.create(&download).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let recent = repo.list_recent(2).await.unwrap();

        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, ids_oldest_to_newest[2]);
        assert_eq!(recent[1].id, ids_oldest_to_newest[1]);
    }
}
