use sqlx::SqlitePool;

use crate::app::env::Env;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub env: Env,
}
