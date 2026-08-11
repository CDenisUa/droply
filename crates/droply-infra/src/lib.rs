//! Concrete implementations of `droply-application` boundaries: Postgres,
//! outbound HTTP, filesystem. Only what Phase 0 needs (DB pool + readiness
//! ping) exists so far — see `docs/CURRENT_STATE.md`.

pub mod postgres;

pub use postgres::{create_pool, create_pool_with_timeout, ping};
