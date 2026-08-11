use std::sync::Arc;

use async_trait::async_trait;
use droply_application::{MediaSourceResolver, UrlValidator};
use droply_domain::DroplyError;
use droply_infra::DirectFileAnalyzer;
use url::Url;

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
