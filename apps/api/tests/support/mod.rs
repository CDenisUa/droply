use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use droply_api::AppDependencies;
use droply_application::{
    DownloadRepository, DownloadStrategyResolver, MediaSourceResolver, UrlValidator,
};
use droply_domain::{Download, DroplyError};
use droply_infra::{DirectFileAnalyzer, DirectFileDownloadStrategy};
use url::Url;
use uuid::Uuid;

/// Real SSRF policy is covered by `droply-infra`'s own tests — these
/// integration tests exercise routing/wiring, so a permissive validator
/// lets them point at a local `wiremock` server (which the real
/// `SsrfSafeUrlValidator` would correctly reject as loopback).
pub struct AllowAllValidator;

#[async_trait]
impl UrlValidator for AllowAllValidator {
    async fn validate(&self, _url: &Url) -> Result<(), DroplyError> {
        Ok(())
    }
}

/// In-memory `DownloadRepository` — these tests exercise HTTP routing and
/// download orchestration, not Postgres itself (that's
/// `droply-infra`'s `download_repository::tests`, run against a live DB).
#[derive(Default)]
pub struct InMemoryDownloadRepository {
    rows: Mutex<HashMap<Uuid, Download>>,
}

#[async_trait]
impl DownloadRepository for InMemoryDownloadRepository {
    async fn create(&self, download: &Download) -> anyhow::Result<()> {
        #[allow(clippy::unwrap_used)]
        self.rows
            .lock()
            .unwrap()
            .insert(download.id, download.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> anyhow::Result<Option<Download>> {
        #[allow(clippy::unwrap_used)]
        Ok(self.rows.lock().unwrap().get(&id).cloned())
    }

    async fn update(&self, download: &Download) -> anyhow::Result<()> {
        #[allow(clippy::unwrap_used)]
        self.rows
            .lock()
            .unwrap()
            .insert(download.id, download.clone());
        Ok(())
    }

    async fn list_recent(&self, limit: i64) -> anyhow::Result<Vec<Download>> {
        #[allow(clippy::unwrap_used)]
        let rows = self.rows.lock().unwrap();
        let mut all: Vec<Download> = rows.values().cloned().collect();
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        #[allow(clippy::cast_sign_loss)]
        all.truncate(limit as usize);
        Ok(all)
    }
}

/// A resolver with no analyzers registered — enough for tests that only
/// exercise `/healthz`/`/readyz`, which never touch the resolver.
#[allow(dead_code)]
pub fn empty_resolver() -> Arc<MediaSourceResolver> {
    Arc::new(MediaSourceResolver::new(vec![]))
}

/// A resolver wired with `DirectFileAnalyzer` behind a permissive
/// validator, for tests exercising `/api/sources/analyze` against a local
/// `wiremock` server.
#[allow(dead_code)]
pub fn direct_file_resolver() -> Arc<MediaSourceResolver> {
    #[allow(clippy::unwrap_used)]
    let analyzer = DirectFileAnalyzer::new(Arc::new(AllowAllValidator)).unwrap();
    Arc::new(MediaSourceResolver::new(vec![Arc::new(analyzer)]))
}

/// A `DownloadStrategyResolver` with no strategies registered — enough for
/// tests that don't exercise `/api/downloads`.
#[allow(dead_code)]
pub fn empty_download_strategy_resolver() -> Arc<DownloadStrategyResolver> {
    Arc::new(DownloadStrategyResolver::new(vec![]))
}

/// A `DownloadStrategyResolver` wired with `DirectFileDownloadStrategy`
/// behind a permissive validator, for tests exercising `/api/downloads`
/// against a local `wiremock` server.
#[allow(dead_code)]
pub fn direct_file_download_strategy_resolver() -> Arc<DownloadStrategyResolver> {
    #[allow(clippy::unwrap_used)]
    let strategy = DirectFileDownloadStrategy::new(Arc::new(AllowAllValidator)).unwrap();
    Arc::new(DownloadStrategyResolver::new(vec![Arc::new(strategy)]))
}

/// Bundles the pieces `app()` needs, with an in-memory download repository
/// and a fresh temp directory per test.
#[allow(dead_code)]
pub fn dependencies(
    source_resolver: Arc<MediaSourceResolver>,
    download_strategy_resolver: Arc<DownloadStrategyResolver>,
) -> AppDependencies {
    AppDependencies {
        source_resolver,
        download_repository: Arc::new(InMemoryDownloadRepository::default()),
        download_strategy_resolver,
        temp_storage_path: std::env::temp_dir().join(format!("droply-test-{}", Uuid::new_v4())),
    }
}
