use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use async_trait::async_trait;
use droply_domain::{DroplyError, SourceType};
use tokio_util::sync::CancellationToken;
use url::Url;

/// How a source's bytes are actually obtained, as opposed to
/// `MediaSourceAnalyzer` which only determines what exists (doc §12).
///
/// `execute` writes the fetched file to `destination` (never buffering the
/// whole thing in memory — AGENTS.md rule 7-8), publishing progress via
/// `bytes_downloaded` (a shared atomic rather than a callback, so the
/// strategy doesn't need to know anything about how/how-often the caller
/// persists progress) and stopping promptly once `cancellation` fires.
#[async_trait]
pub trait DownloadStrategy: Send + Sync {
    fn can_handle(&self, source_type: SourceType) -> bool;

    async fn execute(
        &self,
        source_url: &Url,
        destination: &Path,
        bytes_downloaded: Arc<AtomicU64>,
        cancellation: CancellationToken,
    ) -> Result<(), DroplyError>;
}

/// Tries each registered strategy in order and delegates to the first one
/// that claims the source type — mirrors `MediaSourceResolver`.
pub struct DownloadStrategyResolver {
    strategies: Vec<Arc<dyn DownloadStrategy>>,
}

impl DownloadStrategyResolver {
    pub fn new(strategies: Vec<Arc<dyn DownloadStrategy>>) -> Self {
        Self { strategies }
    }

    pub fn resolve(
        &self,
        source_type: SourceType,
    ) -> Result<Arc<dyn DownloadStrategy>, DroplyError> {
        self.strategies
            .iter()
            .find(|strategy| strategy.can_handle(source_type))
            .cloned()
            .ok_or(DroplyError::UnsupportedSource)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    struct StubStrategy {
        handles: SourceType,
    }

    #[async_trait]
    impl DownloadStrategy for StubStrategy {
        fn can_handle(&self, source_type: SourceType) -> bool {
            source_type == self.handles
        }

        async fn execute(
            &self,
            _source_url: &Url,
            _destination: &Path,
            _bytes_downloaded: Arc<AtomicU64>,
            _cancellation: CancellationToken,
        ) -> Result<(), DroplyError> {
            Ok(())
        }
    }

    #[test]
    fn resolves_to_the_strategy_that_handles_the_source_type() {
        let resolver = DownloadStrategyResolver::new(vec![
            Arc::new(StubStrategy {
                handles: SourceType::Hls,
            }),
            Arc::new(StubStrategy {
                handles: SourceType::DirectFile,
            }),
        ]);

        assert!(resolver.resolve(SourceType::DirectFile).is_ok());
    }

    #[test]
    fn returns_unsupported_source_when_nothing_matches() {
        let resolver = DownloadStrategyResolver::new(vec![Arc::new(StubStrategy {
            handles: SourceType::Hls,
        })]);

        let result = resolver.resolve(SourceType::DirectFile);
        assert!(matches!(result, Err(DroplyError::UnsupportedSource)));
    }
}
