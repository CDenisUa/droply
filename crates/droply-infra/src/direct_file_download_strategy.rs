use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use droply_application::{DownloadStrategy, UrlValidator};
use droply_domain::{DroplyError, SourceType};
use futures_util::StreamExt;
use reqwest::{Client, Method};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::http::request_with_redirects;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Applies per HTTP request/response activity (connecting, headers), not to
/// the download as a whole — a multi-gigabyte file legitimately takes
/// longer than any fixed timeout without any single operation stalling.
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// Streams a direct file from its source URL straight to disk. Never holds
/// the file in memory (AGENTS.md rule 7-8): each chunk is written to
/// `destination` and dropped immediately. Progress is published via a
/// shared atomic rather than persisted here — persistence cadence is the
/// caller's decision, not this strategy's.
pub struct DirectFileDownloadStrategy {
    client: Client,
    validator: Arc<dyn UrlValidator>,
}

impl DirectFileDownloadStrategy {
    pub fn new(validator: Arc<dyn UrlValidator>) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(IDLE_TIMEOUT)
            .build()?;

        Ok(Self { client, validator })
    }
}

#[async_trait]
impl DownloadStrategy for DirectFileDownloadStrategy {
    fn can_handle(&self, source_type: SourceType) -> bool {
        source_type == SourceType::DirectFile
    }

    async fn execute(
        &self,
        source_url: &Url,
        destination: &Path,
        bytes_downloaded: Arc<AtomicU64>,
        cancellation: CancellationToken,
    ) -> Result<(), DroplyError> {
        self.validator.validate(source_url).await?;

        let response = request_with_redirects(
            &self.client,
            self.validator.as_ref(),
            Method::GET,
            source_url.clone(),
        )
        .await?;

        if !response.status().is_success() {
            return Err(DroplyError::SourceUnavailable);
        }

        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|_| DroplyError::InsufficientStorage)?;
        }

        let mut file = File::create(destination)
            .await
            .map_err(|_| DroplyError::InsufficientStorage)?;

        let mut stream = response.bytes_stream();
        let mut total: u64 = 0;

        loop {
            tokio::select! {
                biased;

                () = cancellation.cancelled() => {
                    drop(file);
                    let _ = tokio::fs::remove_file(destination).await;
                    return Err(DroplyError::DownloadCancelled);
                }

                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(bytes)) => {
                            file.write_all(&bytes)
                                .await
                                .map_err(|_| DroplyError::InsufficientStorage)?;
                            total += bytes.len() as u64;
                            bytes_downloaded.store(total, Ordering::Relaxed);
                        }
                        Some(Err(_)) => return Err(DroplyError::SourceUnavailable),
                        None => break,
                    }
                }
            }
        }

        file.flush()
            .await
            .map_err(|_| DroplyError::InsufficientStorage)?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct AllowAllValidator;

    #[async_trait]
    impl UrlValidator for AllowAllValidator {
        async fn validate(&self, _url: &Url) -> Result<(), DroplyError> {
            Ok(())
        }
    }

    fn strategy() -> DirectFileDownloadStrategy {
        DirectFileDownloadStrategy::new(Arc::new(AllowAllValidator)).unwrap()
    }

    fn temp_destination(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("droply-test-{}-{name}", uuid::Uuid::new_v4()))
    }

    #[tokio::test]
    async fn downloads_the_full_body_to_the_destination_file() {
        let server = MockServer::start().await;
        let body = b"the quick brown fox jumps over the lazy dog".repeat(100);
        Mock::given(method("GET"))
            .and(path("/file.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .mount(&server)
            .await;

        let destination = temp_destination("full-body");
        let progress = Arc::new(AtomicU64::new(0));
        let url = Url::parse(&format!("{}/file.bin", server.uri())).unwrap();

        strategy()
            .execute(
                &url,
                &destination,
                progress.clone(),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let written = tokio::fs::read(&destination).await.unwrap();
        assert_eq!(written, body);
        assert_eq!(progress.load(Ordering::Relaxed), body.len() as u64);

        let _ = tokio::fs::remove_file(&destination).await;
    }

    #[tokio::test]
    async fn cancellation_stops_the_download_and_removes_the_partial_file() {
        let server = MockServer::start().await;
        let body = vec![0u8; 10_000_000]; // large enough that cancellation wins the race
        Mock::given(method("GET"))
            .and(path("/big.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
            .mount(&server)
            .await;

        let destination = temp_destination("cancelled");
        let progress = Arc::new(AtomicU64::new(0));
        let url = Url::parse(&format!("{}/big.bin", server.uri())).unwrap();
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        let result = strategy()
            .execute(&url, &destination, progress, cancellation)
            .await;

        assert_eq!(result.unwrap_err(), DroplyError::DownloadCancelled);
        assert!(!destination.exists());
    }

    #[tokio::test]
    async fn returns_source_unavailable_for_a_non_success_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/missing.bin"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let destination = temp_destination("missing");
        let progress = Arc::new(AtomicU64::new(0));
        let url = Url::parse(&format!("{}/missing.bin", server.uri())).unwrap();

        let result = strategy()
            .execute(&url, &destination, progress, CancellationToken::new())
            .await;

        assert_eq!(result.unwrap_err(), DroplyError::SourceUnavailable);
        assert!(!destination.exists());
    }

    #[test]
    fn only_handles_direct_file_sources() {
        let s = strategy();
        assert!(s.can_handle(SourceType::DirectFile));
        assert!(!s.can_handle(SourceType::Hls));
        assert!(!s.can_handle(SourceType::Dash));
        assert!(!s.can_handle(SourceType::LocalFile));
    }
}
