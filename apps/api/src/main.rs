use std::env;
use std::sync::Arc;

use droply_api::{app, cors_layer_from_env};
use droply_application::{MediaSourceAnalyzer, MediaSourceResolver, UrlValidator};
use droply_infra::{DirectFileAnalyzer, SsrfSafeUrlValidator};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let database_url =
        env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;
    let pool = droply_infra::create_pool(&database_url).await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;

    let cors_origins = env::var("CORS_ALLOWED_ORIGINS").ok();
    let cors = cors_layer_from_env(cors_origins.as_deref());

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    // Composition root: decides which UrlValidator and which
    // MediaSourceAnalyzers are actually wired in. More specific analyzers
    // (HLS, DASH) must be registered before DirectFileAnalyzer's catch-all
    // once they exist — see docs/architecture.md §11.
    let url_validator: Arc<dyn UrlValidator> = Arc::new(SsrfSafeUrlValidator::new());
    let analyzers: Vec<Arc<dyn MediaSourceAnalyzer>> =
        vec![Arc::new(DirectFileAnalyzer::new(url_validator)?)];
    let source_resolver = Arc::new(MediaSourceResolver::new(analyzers));

    let router = app(pool, cors, source_resolver);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!(port, "droply-api listening");

    axum::serve(listener, router).await?;
    Ok(())
}
