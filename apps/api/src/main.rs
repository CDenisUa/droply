use std::env;

use droply_api::{app, cors_layer_from_env};

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

    let router = app(pool, cors);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!(port, "droply-api listening");

    axum::serve(listener, router).await?;
    Ok(())
}
