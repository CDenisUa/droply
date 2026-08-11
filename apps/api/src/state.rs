use std::sync::Arc;

use droply_application::MediaSourceResolver;
use sqlx::PgPool;

/// Shared application state handed to route handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub source_resolver: Arc<MediaSourceResolver>,
}
