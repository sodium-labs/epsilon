use crate::environment::{ApiState, Environment};
use axum::{extract::State, http::StatusCode, Json};
use database::{
    models::{NewPageAnalytics, Statistic},
    schema::{pages, pages_analytics, statistics},
    types::StatisticType,
    DbConn,
};
use diesel::{
    dsl::sum, prelude::QueryableByName, sql_query, ExpressionMethods, OptionalExtension, QueryDsl,
    QueryResult, RunQueryDsl,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn create_analytics_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(get_analytics_system_handler))
        .routes(routes!(get_analytics_database_handler))
        .routes(routes!(get_analytics_pages_handler))
        .routes(routes!(post_analytics_click_handler))
}

/// Holds (value, timestamp)
#[derive(utoipa::ToSchema, Serialize)]
struct StatisticValue(i64, i64);

/// Utility function to retrieve sorted statistics
fn get_statistics(
    types: Vec<StatisticType>,
    db_conn: &mut DbConn,
) -> QueryResult<HashMap<StatisticType, Vec<StatisticValue>>> {
    let mut sorted = HashMap::new();

    let results = statistics::table
        .select(statistics::all_columns)
        .filter(statistics::statistic_type.eq_any(types))
        .load::<Statistic>(db_conn)?;

    for r in results {
        sorted
            .entry(r.statistic_type)
            .or_insert(Vec::new())
            .push(StatisticValue(r.value, r.timestamp));
    }

    Ok(sorted)
}

#[derive(utoipa::ToSchema, Serialize)]
struct SystemAnalytics {
    cpu_usages: Vec<StatisticValue>,
    memory_usages: Vec<StatisticValue>,
}

#[utoipa::path(
    get,
    path = "/system",
    description = "Get the system analytics",
    responses(
        (status = OK, body = SystemAnalytics)
    )
)]
#[axum::debug_handler]
async fn get_analytics_system_handler(
    State(state): State<Arc<Environment>>,
) -> Json<SystemAnalytics> {
    let db_conn = &mut state.db_pool.get().unwrap();

    let mut stats = get_statistics(
        vec![StatisticType::MemoryUsage, StatisticType::CpuUsage],
        db_conn,
    )
    .unwrap();

    Json(SystemAnalytics {
        memory_usages: stats
            .remove(&StatisticType::MemoryUsage)
            .unwrap_or(Vec::new()),
        cpu_usages: stats.remove(&StatisticType::CpuUsage).unwrap_or(Vec::new()),
    })
}

#[derive(utoipa::ToSchema, Serialize)]
struct DatabaseAnalytics {
    page_counts: Vec<StatisticValue>,
    indexed_page_counts: Vec<StatisticValue>,
    api_request_counts: Vec<StatisticValue>,
    user_search_counts: Vec<StatisticValue>,
    database_sizes: Vec<StatisticValue>,
    queue_sizes: Vec<StatisticValue>,
    word_counts: Vec<StatisticValue>,
    indexes_counts: Vec<StatisticValue>,
    favicons_counts: Vec<StatisticValue>,
}

#[utoipa::path(
    get,
    path = "/database",
    description = "Get the database analytics",
    responses(
        (status = OK, body = DatabaseAnalytics)
    )
)]
#[axum::debug_handler]
async fn get_analytics_database_handler(
    State(state): State<Arc<Environment>>,
) -> Json<DatabaseAnalytics> {
    let db_conn = &mut state.db_pool.get().unwrap();

    let mut stats = get_statistics(
        vec![
            StatisticType::CrawledPageCount,
            StatisticType::IndexedPageCount,
            StatisticType::ApiRequestCount,
            StatisticType::UserSearchCount,
            StatisticType::DatabaseSize,
            StatisticType::QueueSize,
            StatisticType::WordCount,
            StatisticType::IndexesCount,
            StatisticType::FaviconsCount,
        ],
        db_conn,
    )
    .unwrap();

    Json(DatabaseAnalytics {
        page_counts: stats
            .remove(&StatisticType::CrawledPageCount)
            .unwrap_or(Vec::new()),
        indexed_page_counts: stats
            .remove(&StatisticType::IndexedPageCount)
            .unwrap_or(Vec::new()),
        api_request_counts: stats
            .remove(&StatisticType::ApiRequestCount)
            .unwrap_or(Vec::new()),
        user_search_counts: stats
            .remove(&StatisticType::UserSearchCount)
            .unwrap_or(Vec::new()),
        database_sizes: stats
            .remove(&StatisticType::DatabaseSize)
            .unwrap_or(Vec::new()),
        queue_sizes: stats
            .remove(&StatisticType::QueueSize)
            .unwrap_or(Vec::new()),
        word_counts: stats
            .remove(&StatisticType::WordCount)
            .unwrap_or(Vec::new()),
        indexes_counts: stats
            .remove(&StatisticType::IndexesCount)
            .unwrap_or(Vec::new()),
        favicons_counts: stats
            .remove(&StatisticType::FaviconsCount)
            .unwrap_or(Vec::new()),
    })
}

#[derive(utoipa::ToSchema, Serialize)]
struct PagesAnalytics {
    average_search_time: i64,
    total_clicks: i64,
    total_impressions: i64,
}

#[derive(QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct AvgResult {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
    avg: Option<f64>,
}

#[utoipa::path(
    get,
    path = "/pages",
    description = "Get the pages global analytics",
    responses(
        (status = OK, body = PagesAnalytics)
    )
)]
#[axum::debug_handler]
async fn get_analytics_pages_handler(
    State(state): State<Arc<Environment>>,
) -> Json<PagesAnalytics> {
    let db_conn = &mut state.db_pool.get().unwrap();

    let result: (Option<i64>, Option<i64>) = pages_analytics::table
        .select((
            sum(pages_analytics::clicks),
            sum(pages_analytics::impressions),
        ))
        .first(db_conn)
        .expect("Error calculating sum");

    let average_result = sql_query("SELECT AVG(search_time)::float8 AS avg FROM queries")
        .get_result::<AvgResult>(db_conn)
        .unwrap();

    Json(PagesAnalytics {
        average_search_time: average_result.avg.map(|t| t.floor() as i64).unwrap_or(-1),
        total_clicks: result.0.unwrap_or(-1),
        total_impressions: result.1.unwrap_or(-1),
    })
}

#[derive(Deserialize)]
struct ClickAnalyticsBody {
    page_url: String,
}

#[utoipa::path(
    post,
    path = "/click",
    description = "A page was clicked",
    responses(
        (status = OK),
        (status = BAD_REQUEST)
    )
)]
#[axum::debug_handler]
async fn post_analytics_click_handler(
    State(state): State<Arc<Environment>>,
    Json(payload): Json<ClickAnalyticsBody>,
) -> StatusCode {
    if payload.page_url.len() > 2048 {
        return StatusCode::BAD_REQUEST;
    }

    let db_conn = &mut state.db_pool.get().unwrap();

    if let Some(page_id) = pages::table
        .select(pages::id)
        .filter(pages::url.eq(payload.page_url))
        .get_result::<i32>(db_conn)
        .optional()
        .unwrap()
    {
        diesel::insert_into(pages_analytics::table)
            .values(NewPageAnalytics {
                page_id,
                clicks: 1,
                impressions: 0,
            })
            .on_conflict(pages_analytics::page_id)
            .do_update()
            .set(pages_analytics::clicks.eq(pages_analytics::clicks + 1))
            .execute(db_conn)
            .unwrap();

        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    }
}
