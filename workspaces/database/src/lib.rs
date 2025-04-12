use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sql_query;

pub mod models;
pub mod schema;
pub mod types;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConn = PooledConnection<ConnectionManager<PgConnection>>;

pub const MAX_POOL_SIZE: u32 = 40;

pub fn create_pool(db_url: &str) -> DbPool {
    let manager = ConnectionManager::<PgConnection>::new(db_url);
    Pool::builder()
        .max_size(MAX_POOL_SIZE)
        .build(manager)
        .expect("Failed to create DB pool")
}

#[derive(QueryableByName)]
pub struct TableSize {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub table_name: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub size: i64,
}

pub fn get_table_sizes(conn: &mut PgConnection) -> Vec<TableSize> {
    sql_query(
        "SELECT relname AS table_name, pg_total_relation_size(relid) AS size
         FROM pg_catalog.pg_statio_user_tables
         ORDER BY size DESC",
    )
    .load::<TableSize>(conn)
    .expect("Failed to get the table sizes")
}

#[derive(QueryableByName)]
pub struct DbSize {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub size: i64,
}

pub fn get_database_size(conn: &mut PgConnection) -> QueryResult<i64> {
    let result = sql_query("SELECT pg_database_size(current_database()) AS size")
        .get_result::<DbSize>(conn)?;
    Ok(result.size)
}
