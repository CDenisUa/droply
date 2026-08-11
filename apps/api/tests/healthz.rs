#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use droply_api::{app, cors_layer_from_env};
use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

mod support;

/// `/healthz` must respond without touching the database at all, so this
/// test uses a lazy pool that never actually connects — proving the
/// liveness endpoint has no hidden DB dependency.
#[tokio::test]
async fn healthz_returns_ok_without_a_reachable_database() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://user:pass@127.0.0.1:1/db")
        .expect("connect_lazy must not perform I/O");

    let app = app(
        pool,
        cors_layer_from_env(None),
        support::dependencies(
            support::empty_resolver(),
            support::empty_download_strategy_resolver(),
        ),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}
