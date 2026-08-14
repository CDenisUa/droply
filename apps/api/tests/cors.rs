#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use droply_api::{app, cors_layer_from_env};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

mod support;

fn unused_pool() -> sqlx::PgPool {
    PgPoolOptions::new()
        .connect_lazy("postgres://user:pass@127.0.0.1:1/db")
        .unwrap()
}

/// Guards against a real regression: the frontend's JSON POST requests
/// (`analyzeSource`, `createDownload`, ...) send `Content-Type:
/// application/json`, which is not a CORS-safelisted header value, so the
/// browser sends a preflight `OPTIONS` request first. If the allowed
/// origin doesn't also echo `content-type` back in
/// `Access-Control-Allow-Headers`, the browser blocks the real request
/// before it's ever sent — this only surfaces in an actual browser, not in
/// `oneshot`-style tests that skip preflight, which is exactly how it
/// slipped through until manual browser verification caught it.
#[tokio::test]
async fn preflight_for_a_json_post_allows_the_content_type_header() {
    let app = app(
        unused_pool(),
        cors_layer_from_env(Some("http://localhost:5173")),
        support::dependencies(
            support::empty_resolver(),
            support::empty_download_strategy_resolver(),
        ),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/sources/analyze")
                .header("origin", "http://localhost:5173")
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let allow_headers = response
        .headers()
        .get("access-control-allow-headers")
        .expect("preflight response must echo back the requested headers")
        .to_str()
        .unwrap()
        .to_lowercase();
    assert!(
        allow_headers.contains("content-type"),
        "expected content-type to be allowed, got: {allow_headers}"
    );

    let allow_origin = response
        .headers()
        .get("access-control-allow-origin")
        .expect("preflight response must echo back the allowed origin")
        .to_str()
        .unwrap();
    assert_eq!(allow_origin, "http://localhost:5173");
}
