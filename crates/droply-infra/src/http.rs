use droply_application::UrlValidator;
use droply_domain::DroplyError;
use reqwest::header::LOCATION;
use reqwest::{Client, Method, Response};
use url::Url;

/// Shared by every component that issues outbound HTTP requests to a
/// user-supplied URL (`DirectFileAnalyzer`, `DirectFileDownloadStrategy`,
/// ...). Centralized deliberately: redirect-hop re-validation is a
/// security-relevant behavior (doc §27), and having it in one audited place
/// instead of duplicated per-caller is what keeps it from drifting out of
/// sync if one copy gets edited and the other doesn't.
pub const MAX_REDIRECTS: u8 = 5;

/// Follows redirects manually (the `Client` must be built with
/// `redirect::Policy::none()`) so every hop can be re-validated through
/// `validator` before being followed — see doc §27 ("validation must also
/// cover redirect destinations").
pub async fn request_with_redirects(
    client: &Client,
    validator: &dyn UrlValidator,
    method: Method,
    start_url: Url,
) -> Result<Response, DroplyError> {
    let mut url = start_url;

    for _ in 0..=MAX_REDIRECTS {
        let response = client
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

        validator.validate(&next_url).await?;
        url = next_url;
    }

    Err(DroplyError::SourceUnavailable)
}
