use std::sync::Arc;
use sqlx::MySqlPool;

pub mod accounts;
pub mod rooms;

pub type DatabasePool = Arc<MySqlPool>;