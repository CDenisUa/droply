use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde_json::json;
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Shared application state handed to route handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

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

/// Build the full application router. CORS origins are passed in explicitly
/// (read from `CORS_ALLOWED_ORIGINS` by the caller) rather than defaulting
/// to `AllowAnyOrigin`, per `docs/architecture.md` §55.
pub fn app(pool: PgPool, cors: CorsLayer) -> Router {
    let state = Arc::new(AppState { pool });

    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

/// Parse a comma-separated `CORS_ALLOWED_ORIGINS` value into a `CorsLayer`.
/// Empty/unset means "no cross-origin requests allowed" rather than "allow
/// any origin" — an empty allowlist is the safe default.
pub fn cors_layer_from_env(value: Option<&str>) -> CorsLayer {
    use axum::http::HeaderValue;

    let origins: Vec<HeaderValue> = value
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|origin| origin.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
}
