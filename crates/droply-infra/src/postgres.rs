use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};

/// sqlx's own default acquire timeout is 30s, which is too slow to fail on
/// a misconfigured `DATABASE_URL` in production or in tests.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Create a bounded connection pool. Kept small deliberately — Render's free
/// tier and Neon's free tier both have low default connection limits.
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    create_pool_with_timeout(database_url, DEFAULT_CONNECT_TIMEOUT).await
}

/// Same as [`create_pool`] with an explicit acquire timeout — mainly so
/// tests exercising the "database unreachable" path fail in ~1s instead of
/// waiting out the production timeout.
pub async fn create_pool_with_timeout(
    database_url: &str,
    timeout: Duration,
) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(timeout)
        .connect(database_url)
        .await
}

/// Readiness check: a trivial round-trip query, not a schema check.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_pool_rejects_an_unreachable_database_without_hanging() {
        // Port 1 is reserved and nothing listens there. Some sandboxed
        // network environments silently drop the connection instead of
        // refusing it, so pin a short acquire timeout rather than relying on
        // the OS to refuse fast — this keeps the test itself fast.
        let result = create_pool_with_timeout(
            "postgres://user:pass@127.0.0.1:1/db",
            Duration::from_secs(1),
        )
        .await;
        assert!(result.is_err());
    }
}
