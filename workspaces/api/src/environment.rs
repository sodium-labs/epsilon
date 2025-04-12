use database::DbPool;
use std::sync::Arc;

pub struct Environment {
    pub db_pool: DbPool,
}

pub type ApiState = Arc<Environment>;
