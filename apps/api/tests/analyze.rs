#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use droply_api::{app, cors_layer_from_env};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod support;

fn unused_pool() -> sqlx::PgPool {
    #[allow(clippy::unwrap_used)]
    PgPoolOptions::new()
        .connect_lazy("postgres://user:pass@127.0.0.1:1/db")
        .unwrap()
}

async fn post_analyze(app: axum::Router, url: &str) -> (StatusCode, Value) {
    let body = Body::from(json!({ "url": url }).to_string());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sources/analyze")
                .header("content-type", "application/json")
                .body(body)
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

#[tokio::test]
async fn analyzes_a_direct_file_and_returns_its_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/song.mp3"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "audio/mpeg")
                .insert_header("content-length", "4096"),
        )
        .mount(&server)
        .await;

    let app = app(
        unused_pool(),
        cors_layer_from_env(None),
        support::dependencies(
            support::direct_file_resolver(),
            support::empty_download_strategy_resolver(),
        ),
    );

    let (status, body) = post_analyze(app, &format!("{}/song.mp3", server.uri())).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sourceType"], "directFile");
    assert_eq!(body["title"], "song.mp3");
    assert_eq!(body["mimeType"], "audio/mpeg");
    assert_eq!(body["sizeBytes"], 4096);
}

#[tokio::test]
async fn rejects_a_malformed_url_with_400() {
    let app = app(
        unused_pool(),
        cors_layer_from_env(None),
        support::dependencies(
            support::direct_file_resolver(),
            support::empty_download_strategy_resolver(),
        ),
    );

    let (status, body) = post_analyze(app, "not a url").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_url");
}

#[tokio::test]
async fn returns_bad_gateway_when_the_source_is_unreachable() {
    // A 404 from both HEAD and GET, not an actually-dead address — this
    // keeps the test fast/deterministic instead of depending on how long a
    // real connection attempt takes to time out (see docs/CURRENT_STATE.md
    // on sandboxed networking sometimes dropping rather than refusing).
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

    let app = app(
        unused_pool(),
        cors_layer_from_env(None),
        support::dependencies(
            support::direct_file_resolver(),
            support::empty_download_strategy_resolver(),
        ),
    );

    let (status, body) = post_analyze(app, &format!("{}/missing.mp4", server.uri())).await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(body["error"], "source_unavailable");
}

/// Uses the real app-building path but relies on the real `UrlValidator`
/// only being reachable through `main.rs`'s composition, not through this
/// test's injected `direct_file_resolver()` — so this test instead proves
/// the SSRF check happens at the `DirectFileAnalyzer` layer by using a
/// resolver built the same way `main.rs` builds its production one.
#[tokio::test]
async fn rejects_a_private_ip_target_via_the_real_ssrf_validator() {
    use droply_application::{MediaSourceAnalyzer, MediaSourceResolver};
    use droply_infra::{DirectFileAnalyzer, SsrfSafeUrlValidator};
    use std::sync::Arc;

    let validator: Arc<dyn droply_application::UrlValidator> =
        Arc::new(SsrfSafeUrlValidator::new());
    let analyzer: Arc<dyn MediaSourceAnalyzer> =
        Arc::new(DirectFileAnalyzer::new(validator).unwrap());
    let resolver = Arc::new(MediaSourceResolver::new(vec![analyzer]));

    let app = app(
        unused_pool(),
        cors_layer_from_env(None),
        support::dependencies(resolver, support::empty_download_strategy_resolver()),
    );

    let (status, body) = post_analyze(app, "http://169.254.169.254/latest/meta-data/").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_url");
}
