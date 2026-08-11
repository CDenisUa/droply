use std::sync::Arc;

use async_trait::async_trait;
use droply_domain::{DroplyError, MediaSourceResult};
use url::Url;

/// One source type's ability to recognize and analyze a URL. Adding a new
/// source type means implementing this trait, not editing an existing
/// analyzer or the resolver — see `docs/architecture.md` §11 and AGENTS.md
/// rule 11.
#[async_trait]
pub trait MediaSourceAnalyzer: Send + Sync {
    async fn can_handle(&self, url: &Url) -> bool;
    async fn analyze(&self, url: &Url) -> Result<MediaSourceResult, DroplyError>;
}

/// Tries each registered analyzer in order and delegates to the first one
/// that claims the URL. Order matters: more specific analyzers (HLS, DASH)
/// must be registered before a catch-all like `DirectFileAnalyzer`.
pub struct MediaSourceResolver {
    analyzers: Vec<Arc<dyn MediaSourceAnalyzer>>,
}

impl MediaSourceResolver {
    pub fn new(analyzers: Vec<Arc<dyn MediaSourceAnalyzer>>) -> Self {
        Self { analyzers }
    }

    pub async fn resolve(&self, url: &Url) -> Result<MediaSourceResult, DroplyError> {
        for analyzer in &self.analyzers {
            if analyzer.can_handle(url).await {
                return analyzer.analyze(url).await;
            }
        }
        Err(DroplyError::UnsupportedSource)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use droply_domain::SourceType;

    struct StubAnalyzer {
        handles: bool,
        result: MediaSourceResult,
    }

    #[async_trait]
    impl MediaSourceAnalyzer for StubAnalyzer {
        async fn can_handle(&self, _url: &Url) -> bool {
            self.handles
        }

        async fn analyze(&self, _url: &Url) -> Result<MediaSourceResult, DroplyError> {
            Ok(self.result.clone())
        }
    }

    fn sample_result(title: &str) -> MediaSourceResult {
        MediaSourceResult {
            source_type: SourceType::DirectFile,
            title: title.to_string(),
            mime_type: None,
            size_bytes: None,
            duration_seconds: None,
        }
    }

    #[tokio::test]
    async fn delegates_to_the_first_analyzer_that_can_handle_the_url() {
        let resolver = MediaSourceResolver::new(vec![
            Arc::new(StubAnalyzer {
                handles: false,
                result: sample_result("should not be picked"),
            }),
            Arc::new(StubAnalyzer {
                handles: true,
                result: sample_result("picked"),
            }),
        ]);

        let url = Url::parse("https://example.com/file.mp4").unwrap();
        let result = resolver.resolve(&url).await.unwrap();

        assert_eq!(result.title, "picked");
    }

    #[tokio::test]
    async fn returns_unsupported_source_when_no_analyzer_claims_the_url() {
        let resolver = MediaSourceResolver::new(vec![Arc::new(StubAnalyzer {
            handles: false,
            result: sample_result("unused"),
        })]);

        let url = Url::parse("https://example.com/file.mp4").unwrap();
        let result = resolver.resolve(&url).await;

        assert_eq!(result.unwrap_err(), DroplyError::UnsupportedSource);
    }
}
