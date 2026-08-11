use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use droply_api::{app, cors_layer_from_env, AppDependencies};
use droply_application::{
    DownloadRepository, DownloadStrategy, DownloadStrategyResolver, MediaSourceAnalyzer,
    MediaSourceResolver, UrlValidator,
};
use droply_infra::{
    DirectFileAnalyzer, DirectFileDownloadStrategy, PostgresDownloadRepository,
    SsrfSafeUrlValidator,
};

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

    let temp_storage_path: PathBuf = env::var("TEMP_STORAGE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("droply"));

    // Composition root: decides which UrlValidator, MediaSourceAnalyzers,
    // and DownloadStrategys are actually wired in. More specific
    // analyzers/strategies (HLS, DASH) must be registered before
    // DirectFile's catch-all once they exist — see docs/architecture.md §11.
    let url_validator: Arc<dyn UrlValidator> = Arc::new(SsrfSafeUrlValidator::new());

    let analyzers: Vec<Arc<dyn MediaSourceAnalyzer>> =
        vec![Arc::new(DirectFileAnalyzer::new(url_validator.clone())?)];
    let source_resolver = Arc::new(MediaSourceResolver::new(analyzers));

    let strategies: Vec<Arc<dyn DownloadStrategy>> =
        vec![Arc::new(DirectFileDownloadStrategy::new(url_validator)?)];
    let download_strategy_resolver = Arc::new(DownloadStrategyResolver::new(strategies));

    let download_repository: Arc<dyn DownloadRepository> =
        Arc::new(PostgresDownloadRepository::new(pool.clone()));

    let router = app(
        pool,
        cors,
        AppDependencies {
            source_resolver,
            download_repository,
            download_strategy_resolver,
            temp_storage_path,
        },
    );
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!(port, "droply-api listening");

    axum::serve(listener, router).await?;
    Ok(())
}
