use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::state::AppState;

/// Liveness endpoint: the process is up and serving requests. Deliberately
/// has no dependency on the database or any other external system — a
/// load balancer/orchestrator uses this to decide whether to keep routing
/// traffic to this instance at all.
async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

/// Readiness endpoint: the process is up *and* its dependencies (currently
/// just Postgres) are reachable. Returns 503 rather than panicking when the
/// database is unreachable, since "database is down" is an expected,
/// recoverable operating condition, not a bug.
async fn readyz(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match droply_infra::ping(&state.pool).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "status": "ok", "database": "ok" })),
        ),
        Err(err) => {
            tracing::warn!(error = %err, "readiness check failed: database unreachable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "unavailable", "database": "unreachable" })),
            )
        }
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
}
