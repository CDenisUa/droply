use async_trait::async_trait;
use droply_domain::DroplyError;
use url::Url;

/// Every user-provided URL must pass through an implementation of this
/// trait before Droply does anything with it (analysis, download, or a
/// redirect hop encountered while doing either) — see AGENTS.md rule 9 and
/// `docs/architecture.md` §27 (SSRF protection).
///
/// This only decides "is it safe to connect to this destination" (protocol
/// + IP/DNS safety). Per-request bounds (timeouts, redirect count, response
/// size caps) are a separate concern, owned by the HTTP client
/// configuration that issues the actual request.
#[async_trait]
pub trait UrlValidator: Send + Sync {
    async fn validate(&self, url: &Url) -> Result<(), DroplyError>;
}
