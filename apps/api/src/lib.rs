use std::sync::Arc;

use axum::Router;
use droply_application::MediaSourceResolver;
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub mod error;
pub mod routes;
pub mod state;

pub use state::AppState;

/// Build the full application router given already-constructed
/// dependencies. Deliberately takes a `MediaSourceResolver` rather than
/// building one itself — *which* analyzers and *which* `UrlValidator` to
/// use is a composition-root decision (see `main.rs`), not something this
/// router-builder should hardcode. That also makes it trivial for tests to
/// swap in a resolver backed by a permissive validator instead of the real
/// SSRF-checking one.
///
/// CORS origins are passed in explicitly (read from `CORS_ALLOWED_ORIGINS`
/// by the caller) rather than defaulting to `AllowAnyOrigin`, per
/// `docs/architecture.md` §55.
pub fn app(pool: PgPool, cors: CorsLayer, source_resolver: Arc<MediaSourceResolver>) -> Router {
    let state = Arc::new(AppState {
        pool,
        source_resolver,
    });

    Router::new()
        .merge(routes::health::router())
        .merge(routes::sources::router())
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
