#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use droply_api::{app, cors_layer_from_env};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// Requires a real, reachable Postgres — run `docker compose up -d postgres`
/// locally and set `DATABASE_URL`, or rely on CI's postgres service.
/// `cargo test -- --include-ignored` to run it.
#[tokio::test]
#[ignore = "requires DATABASE_URL pointing at a live Postgres"]
async fn readyz_returns_ok_when_database_is_reachable() {
    let database_url =
        std::env::var("DATABASE_URL").expect("set DATABASE_URL to a reachable Postgres");
    let pool = droply_infra::create_pool(&database_url)
        .await
        .expect("failed to connect to DATABASE_URL");

    let app = app(pool, cors_layer_from_env(None));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["database"], "ok");
}

#[tokio::test]
async fn readyz_returns_service_unavailable_when_database_is_unreachable() {
    // `connect_lazy` defers connecting, but the /readyz handler's ping still
    // has to attempt one — pin a short acquire timeout so an environment
    // that silently drops (rather than refuses) the connection doesn't make
    // this test wait out sqlx's 30s default.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(1))
        .connect_lazy("postgres://user:pass@127.0.0.1:1/db")
        .expect("connect_lazy must not perform I/O");

    let app = app(pool, cors_layer_from_env(None));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
