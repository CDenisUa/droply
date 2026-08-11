use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use droply_application::{MediaSourceAnalyzer, UrlValidator};
use droply_domain::{derive_filename, DroplyError, MediaSourceResult, SourceType};
use reqwest::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE, LOCATION};
use reqwest::{Client, Method};
use url::Url;

const MAX_REDIRECTS: u8 = 5;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Bounds header-only requests (HEAD, and the GET fallback whose body we
/// never read) — not a bound on an actual file download, which doesn't
/// exist yet (that's `DirectFileDownloadStrategy`, Phase 1d).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Analyzes plain `http`/`https` URLs that serve a single file directly —
/// the doc's "Direct File" source type (§13). HLS/DASH manifests are
/// declined via `can_handle` so a future `HlsAnalyzer`/`DashAnalyzer` (or,
/// until those exist, an honest `UnsupportedSource` error) handles them
/// instead of this analyzer silently mis-labeling them.
pub struct DirectFileAnalyzer {
    client: Client,
    validator: Arc<dyn UrlValidator>,
}

impl DirectFileAnalyzer {
    pub fn new(validator: Arc<dyn UrlValidator>) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            // Redirects are followed manually so every hop can be
            // re-validated through `validator` — see docs/architecture.md
            // §27 ("validation must also cover redirect destinations").
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;

        Ok(Self { client, validator })
    }

    async fn request_with_redirects(
        &self,
        method: Method,
        start_url: Url,
    ) -> Result<reqwest::Response, DroplyError> {
        let mut url = start_url;

        for _ in 0..=MAX_REDIRECTS {
            let response = self
                .client
                .request(method.clone(), url.clone())
                .send()
                .await
                .map_err(|_| DroplyError::SourceUnavailable)?;

            if !response.status().is_redirection() {
                return Ok(response);
            }

            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(DroplyError::SourceUnavailable)?;

            let next_url = url
                .join(location)
                .map_err(|_| DroplyError::SourceUnavailable)?;

            self.validator.validate(&next_url).await?;
            url = next_url;
        }

        Err(DroplyError::SourceUnavailable)
    }
}

#[async_trait]
impl MediaSourceAnalyzer for DirectFileAnalyzer {
    async fn can_handle(&self, url: &Url) -> bool {
        let path = url.path().to_ascii_lowercase();
        !(path.ends_with(".m3u8") || path.ends_with(".mpd"))
    }

    async fn analyze(&self, url: &Url) -> Result<MediaSourceResult, DroplyError> {
        self.validator.validate(url).await?;

        let head_response = self.request_with_redirects(Method::HEAD, url.clone()).await;

        let response = match head_response {
            Ok(response) if response.status().is_success() => response,
            _ => {
                let get_response = self
                    .request_with_redirects(Method::GET, url.clone())
                    .await?;
                if !get_response.status().is_success() {
                    return Err(DroplyError::SourceUnavailable);
                }
                get_response
                // The body is intentionally never read — dropping the
                // response here closes the connection without buffering
                // it, per AGENTS.md rule 7 (never load a full file into
                // memory, and this isn't even meant to be a download).
            }
        };

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        let content_length = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let content_disposition = response
            .headers()
            .get(CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok());

        let title = derive_filename(content_disposition, response.url().path());

        Ok(MediaSourceResult {
            source_type: SourceType::DirectFile,
            title,
            mime_type: content_type,
            size_bytes: content_length,
            duration_seconds: None,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Test double: real SSRF policy is exercised in
    /// `url_validator::tests` — these tests exercise `DirectFileAnalyzer`'s
    /// own HTTP handling, so a permissive validator keeps them focused (and
    /// able to point at wiremock's `127.0.0.1` server, which the real
    /// `SsrfSafeUrlValidator` would correctly reject as loopback).
    struct AllowAllValidator;

    #[async_trait]
    impl UrlValidator for AllowAllValidator {
        async fn validate(&self, _url: &Url) -> Result<(), DroplyError> {
            Ok(())
        }
    }

    fn analyzer() -> DirectFileAnalyzer {
        DirectFileAnalyzer::new(Arc::new(AllowAllValidator)).unwrap()
    }

    #[tokio::test]
    async fn can_handle_declines_hls_and_dash_manifests() {
        let analyzer = analyzer();
        assert!(
            !analyzer
                .can_handle(&Url::parse("https://example.com/master.m3u8").unwrap())
                .await
        );
        assert!(
            !analyzer
                .can_handle(&Url::parse("https://example.com/manifest.mpd").unwrap())
                .await
        );
        assert!(
            analyzer
                .can_handle(&Url::parse("https://example.com/movie.mp4").unwrap())
                .await
        );
    }

    #[tokio::test]
    async fn analyzes_a_direct_file_via_head() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/movie.mp4"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "video/mp4")
                    .insert_header("content-length", "1048576")
                    .insert_header("content-disposition", r#"attachment; filename="Movie.mp4""#),
            )
            .mount(&server)
            .await;

        let url = Url::parse(&format!("{}/movie.mp4", server.uri())).unwrap();
        let result = analyzer().analyze(&url).await.unwrap();

        assert_eq!(result.source_type, SourceType::DirectFile);
        assert_eq!(result.title, "Movie.mp4");
        assert_eq!(result.mime_type.as_deref(), Some("video/mp4"));
        assert_eq!(result.size_bytes, Some(1_048_576));
    }

    #[tokio::test]
    async fn falls_back_to_get_when_head_is_not_allowed() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/clip.mov"))
            .respond_with(ResponseTemplate::new(405))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/clip.mov"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "video/quicktime")
                    .set_body_bytes(b"not the real body, just proving we don't need it".to_vec()),
            )
            .mount(&server)
            .await;

        let url = Url::parse(&format!("{}/clip.mov", server.uri())).unwrap();
        let result = analyzer().analyze(&url).await.unwrap();

        assert_eq!(result.mime_type.as_deref(), Some("video/quicktime"));
        assert_eq!(result.title, "clip.mov");
    }

    #[tokio::test]
    async fn follows_a_redirect_and_uses_the_final_url_for_the_filename() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/short-link"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("location", format!("{}/real-file.zip", server.uri())),
            )
            .mount(&server)
            .await;
        Mock::given(method("HEAD"))
            .and(path("/real-file.zip"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("content-type", "application/zip"),
            )
            .mount(&server)
            .await;

        let url = Url::parse(&format!("{}/short-link", server.uri())).unwrap();
        let result = analyzer().analyze(&url).await.unwrap();

        assert_eq!(result.title, "real-file.zip");
    }

    #[tokio::test]
    async fn gives_up_after_too_many_redirects() {
        let server = MockServer::start().await;
        // Every hop redirects to itself — an infinite redirect loop.
        Mock::given(method("HEAD"))
            .and(path("/loop"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("location", format!("{}/loop", server.uri())),
            )
            .mount(&server)
            .await;

        let url = Url::parse(&format!("{}/loop", server.uri())).unwrap();
        let result = analyzer().analyze(&url).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn returns_source_unavailable_for_a_404() {
        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/missing.mp4"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/missing.mp4"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let url = Url::parse(&format!("{}/missing.mp4", server.uri())).unwrap();
        let result = analyzer().analyze(&url).await;

        assert_eq!(result.unwrap_err(), DroplyError::SourceUnavailable);
    }
}
