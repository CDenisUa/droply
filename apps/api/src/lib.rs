use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::Router;
use droply_application::{DownloadRepository, DownloadStrategyResolver, MediaSourceResolver};
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub mod download_runner;
pub mod error;
pub mod routes;
pub mod state;

pub use state::AppState;

/// Everything `app()` needs beyond the DB pool and CORS policy — bundled so
/// the function signature doesn't grow a parameter per feature. *Which*
/// concrete analyzers/strategies/validator to use is a composition-root
/// decision (see `main.rs`), not something this router-builder should
/// hardcode — that also makes it trivial for tests to swap in dependencies
/// backed by a permissive validator instead of the real SSRF-checking one.
pub struct AppDependencies {
    pub source_resolver: Arc<MediaSourceResolver>,
    pub download_repository: Arc<dyn DownloadRepository>,
    pub download_strategy_resolver: Arc<DownloadStrategyResolver>,
    pub temp_storage_path: PathBuf,
}

/// Build the full application router given already-constructed
/// dependencies. CORS origins are passed in explicitly (read from
/// `CORS_ALLOWED_ORIGINS` by the caller) rather than defaulting to
/// `AllowAnyOrigin`, per `docs/architecture.md` §55.
pub fn app(pool: PgPool, cors: CorsLayer, deps: AppDependencies) -> Router {
    let state = Arc::new(AppState {
        pool,
        source_resolver: deps.source_resolver,
        download_repository: deps.download_repository,
        download_strategy_resolver: deps.download_strategy_resolver,
        active_cancellations: Arc::new(Mutex::new(std::collections::HashMap::new())),
        temp_storage_path: deps.temp_storage_path,
    });

    Router::new()
        .merge(routes::health::router())
        .merge(routes::sources::router())
        .merge(routes::downloads::router())
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
        .allow_headers([axum::http::header::CONTENT_TYPE])
}
