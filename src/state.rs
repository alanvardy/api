use sqlx::SqlitePool;

use crate::env::Env;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub env: Env,
}
