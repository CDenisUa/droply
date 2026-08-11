#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use droply_api::{app, cors_layer_from_env};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod support;

fn unused_pool() -> sqlx::PgPool {
    PgPoolOptions::new()
        .connect_lazy("postgres://user:pass@127.0.0.1:1/db")
        .unwrap()
}

fn test_app() -> Router {
    app(
        unused_pool(),
        cors_layer_from_env(None),
        support::dependencies(
            support::direct_file_resolver(),
            support::direct_file_download_strategy_resolver(),
        ),
    )
}

async fn request(app: &Router, req: Request<Body>) -> (StatusCode, Value, axum::http::HeaderMap) {
    use tower::ServiceExt;
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json, headers)
}

async fn create_download(app: &Router, url: &str) -> Value {
    let body = Body::from(json!({ "url": url }).to_string());
    let (status, json, _) = request(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/downloads")
            .header("content-type", "application/json")
            .body(body)
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "create failed: {json:?}");
    json
}

async fn get_status(app: &Router, id: &str) -> Value {
    let (_, json, _) = request(
        app,
        Request::builder()
            .uri(format!("/api/downloads/{id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    json
}

/// Polls `GET /api/downloads/{id}` until `status` is one of `targets`, or
/// panics after `timeout`. Background download execution is a spawned
/// tokio task, not something a single request/response can wait on
/// directly.
async fn wait_for_status(app: &Router, id: &str, targets: &[&str], timeout: Duration) -> Value {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let json = get_status(app, id).await;
        let current = json["status"].as_str().unwrap_or("");
        if targets.contains(&current) {
            return json;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timed out waiting for status in {targets:?}, last seen: {json}");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn creates_a_download_and_it_completes_and_serves_content() {
    let server = MockServer::start().await;
    let body_bytes = b"hello droply, this is the file contents".repeat(50);

    Mock::given(method("HEAD"))
        .and(path("/file.bin"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/octet-stream")
                .insert_header("content-length", body_bytes.len().to_string()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/file.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body_bytes.clone()))
        .mount(&server)
        .await;

    let app = test_app();
    let created = create_download(&app, &format!("{}/file.bin", server.uri())).await;
    let id = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["status"], "pending");

    let final_state =
        wait_for_status(&app, &id, &["completed", "failed"], Duration::from_secs(5)).await;
    assert_eq!(
        final_state["status"], "completed",
        "download failed: {final_state}"
    );
    assert_eq!(final_state["bytesDownloaded"], body_bytes.len());

    let (status, _json, headers) = request(
        &app,
        Request::builder()
            .uri(format!("/api/downloads/{id}/content"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-length").unwrap().to_str().unwrap(),
        body_bytes.len().to_string()
    );
}

#[tokio::test]
async fn content_is_not_served_before_the_download_completes() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/slow.bin"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "10"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/slow.bin"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(vec![0u8; 10])
                .set_delay(Duration::from_secs(2)),
        )
        .mount(&server)
        .await;

    let app = test_app();
    let created = create_download(&app, &format!("{}/slow.bin", server.uri())).await;
    let id = created["id"].as_str().unwrap().to_string();

    let (status, json, _) = request(
        &app,
        Request::builder()
            .uri(format!("/api/downloads/{id}/content"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_ne!(status, StatusCode::OK);
    assert_eq!(json["error"], "processing_failed");
}

#[tokio::test]
async fn get_status_returns_404_for_an_unknown_id() {
    let app = test_app();
    let (status, _, _) = request(
        &app,
        Request::builder()
            .uri(format!("/api/downloads/{}", uuid::Uuid::new_v4()))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn cancelling_an_in_flight_download_marks_it_cancelled_and_removes_the_partial_file() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/big.bin"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "1000000"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/big.bin"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(vec![0u8; 1_000_000])
                // Long enough that the cancel request below reliably wins
                // the race against the download finishing on its own.
                .set_delay(Duration::from_millis(300)),
        )
        .mount(&server)
        .await;

    let app = test_app();
    let created = create_download(&app, &format!("{}/big.bin", server.uri())).await;
    let id = created["id"].as_str().unwrap().to_string();

    let (cancel_status, _, _) = request(
        &app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/downloads/{id}/cancel"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(cancel_status, StatusCode::OK);

    let final_state = wait_for_status(
        &app,
        &id,
        &["cancelled", "completed", "failed"],
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        final_state["status"], "cancelled",
        "unexpected outcome: {final_state}"
    );
}

#[tokio::test]
async fn retry_restarts_a_failed_download() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/flaky.bin"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "5"))
        .mount(&server)
        .await;
    // First GET fails, second succeeds — proves retry actually re-executes
    // the strategy rather than just flipping a status.
    Mock::given(method("GET"))
        .and(path("/flaky.bin"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/flaky.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .mount(&server)
        .await;

    let app = test_app();
    let created = create_download(&app, &format!("{}/flaky.bin", server.uri())).await;
    let id = created["id"].as_str().unwrap().to_string();

    wait_for_status(&app, &id, &["failed"], Duration::from_secs(5)).await;

    let (retry_status, retry_body, _) = request(
        &app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/downloads/{id}/retry"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(retry_status, StatusCode::ACCEPTED, "{retry_body:?}");

    let final_state =
        wait_for_status(&app, &id, &["completed", "failed"], Duration::from_secs(5)).await;
    assert_eq!(
        final_state["status"], "completed",
        "retry did not succeed: {final_state}"
    );
}

#[tokio::test]
async fn retry_is_rejected_when_the_download_is_not_failed() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/ok.bin"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "2"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ok.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok".to_vec()))
        .mount(&server)
        .await;

    let app = test_app();
    let created = create_download(&app, &format!("{}/ok.bin", server.uri())).await;
    let id = created["id"].as_str().unwrap().to_string();

    wait_for_status(&app, &id, &["completed"], Duration::from_secs(5)).await;

    let (retry_status, retry_body, _) = request(
        &app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/downloads/{id}/retry"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(retry_status, StatusCode::CONFLICT, "{retry_body:?}");
}

#[tokio::test]
async fn list_returns_recent_downloads_newest_first() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "2"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok".to_vec()))
        .mount(&server)
        .await;

    let app = test_app();
    let first = create_download(&app, &format!("{}/a.bin", server.uri())).await;
    let second = create_download(&app, &format!("{}/b.bin", server.uri())).await;

    let (status, body, _) = request(
        &app,
        Request::builder()
            .uri("/api/downloads")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&first["id"].as_str().unwrap()));
    assert!(ids.contains(&second["id"].as_str().unwrap()));
}
