use crate::environment::{ApiState, Environment};
use axum::{extract::State, Json};
use database::{get_database_size, get_table_sizes};
use diesel::{prelude::QueryableByName, RunQueryDsl};
use serde::Serialize;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn create_statistics_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new().routes(routes!(get_statistics_database_handler))
}

#[derive(utoipa::ToSchema, Serialize)]
struct TableSize {
    name: String,
    size: i64,
}

#[derive(QueryableByName)]
struct SqlStats {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    queue_size: i64,

    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::BigInt>)]
    oldest_queue_element: Option<i64>,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    page_count: i64,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    indexed_page_count: i64,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    word_count: i64,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    index_count: i64,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    vote_count: i64,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    analytic_count: i64,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    query_count: i64,
}

#[derive(utoipa::ToSchema, Serialize)]
struct Statistics {
    database_size: i64,
    tables_size: Vec<TableSize>,
    queue_size: i64,
    page_count: i64,
    indexed_page_count: i64,
    word_count: i64,
    index_count: i64,
    oldest_queue_element: Option<i64>,
    vote_count: i64,
    analytic_count: i64,
    query_count: i64,
}

#[utoipa::path(
    get,
    path = "/database",
    description = "Get the database statistics",
    responses(
        (status = OK, body = Statistics)
    )
)]
#[axum::debug_handler]
async fn get_statistics_database_handler(
    State(state): State<Arc<Environment>>,
) -> Json<Statistics> {
    let db_conn = &mut state.db_pool.get().unwrap();

    let stats = diesel::sql_query(
        "SELECT 
            (SELECT COUNT(*) FROM queue) AS queue_size,
            (SELECT timestamp FROM queue ORDER BY timestamp ASC LIMIT 1) AS oldest_queue_element,
            (SELECT COUNT(*) FROM pages) AS page_count,
            (SELECT COUNT(*) FROM pages WHERE last_indexed IS NOT NULL) AS indexed_page_count,
            (SELECT COUNT(*) FROM words) AS word_count,
            (SELECT COUNT(*) FROM indexes) AS index_count,
            (SELECT COUNT(*) FROM votes) AS vote_count,
            (SELECT COUNT(*) FROM statistics) AS analytic_count,
            (SELECT COUNT(*) FROM queries) AS query_count",
    )
    .get_result::<SqlStats>(db_conn)
    .unwrap();

    let tables_size = get_table_sizes(db_conn);
    let database_size = get_database_size(db_conn).unwrap();

    Json(Statistics {
        database_size,
        tables_size: tables_size
            .iter()
            .filter(|t| !t.table_name.starts_with("__"))
            .map(|t| TableSize {
                name: t.table_name.clone(),
                size: t.size,
            })
            .collect(),
        queue_size: stats.queue_size,
        page_count: stats.page_count,
        indexed_page_count: stats.indexed_page_count,
        word_count: stats.word_count,
        index_count: stats.index_count,
        oldest_queue_element: stats.oldest_queue_element,
        vote_count: stats.vote_count,
        analytic_count: stats.analytic_count,
        query_count: stats.query_count,
    })
}
