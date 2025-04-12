use crate::environment::{ApiState, Environment};
use axum::{
    extract::{Query, State},
    http::{self, header::USER_AGENT, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use database::{
    models::{NewPageAnalytics, NewQuery, NewQueuedPage, Page, PageAnalytics, Word},
    schema::{indexes, pages, pages_analytics, queries, queue, words},
    DbConn,
};
use diesel::{
    dsl::sql, prelude::QueryableByName, sql_query, BoolExpressionMethods, BoxableExpression,
    ExpressionMethods, JoinOnDsl, OptionalExtension, QueryDsl, QueryResult, RunQueryDsl,
    TextExpressionMethods,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, sync::Arc, time::Instant};
use utils::{safe_slice, sql::get_sql_timestamp, url::normalize_url};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn create_base_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(get_ping_handler))
        .routes(routes!(get_search_handler))
        .routes(routes!(post_request_url_handler))
}

#[utoipa::path(
    get,
    path = "/ping",
    description = "Ping the API",
    responses(
        (status = OK)
    )
)]
#[axum::debug_handler]
async fn get_ping_handler() -> StatusCode {
    StatusCode::OK
}

#[derive(Deserialize)]
struct RequestUrlBody {
    url: String,
}

#[utoipa::path(
    post,
    path = "/request-url",
    description = "Add a new url to the queue. The 'Authorization' header with your API_KEY is required",
    responses(
        (status = PROCESSING, description = "URL already in the queue"),
        (status = CREATED, description = "URL already crawled (not necessarily indexed)"),
        (status = ACCEPTED, description = "URL added to the queue"),
        (status = BAD_REQUEST, description = "Invalid URL"),
    )
)]
#[axum::debug_handler]
async fn post_request_url_handler(
    headers: HeaderMap,
    State(state): State<Arc<Environment>>,
    Json(payload): Json<RequestUrlBody>,
) -> StatusCode {
    if let Ok(api_key) = env::var("API_KEY") {
        if let Some(authorization) = headers.get(http::header::AUTHORIZATION) {
            if authorization.to_str().ok().unwrap_or("None") != api_key {
                return StatusCode::UNAUTHORIZED;
            }
        } else {
            return StatusCode::UNAUTHORIZED;
        }
    } else {
        eprintln!("[API] API_KEY not found in .env");
        return StatusCode::UNAUTHORIZED;
    }

    let db_conn = &mut state.db_pool.get().unwrap();

    if let Some((url, domain)) = normalize_url(&payload.url) {
        if url.to_string().len() > 1024 {
            return StatusCode::BAD_REQUEST;
        }

        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return StatusCode::BAD_REQUEST;
        }

        let in_queue = queue::table
            .filter(queue::url.eq(url.to_string()))
            .select(queue::id)
            .first::<i32>(db_conn)
            .optional()
            .expect("Error checking queue");

        if in_queue.is_some() {
            return StatusCode::PROCESSING;
        }

        let in_pages = pages::table
            .filter(pages::url.eq(url.to_string()))
            .select(pages::id)
            .first::<i32>(db_conn)
            .optional()
            .expect("Error checking pages");

        if in_pages.is_some() {
            return StatusCode::CREATED;
        }

        let new_element = NewQueuedPage {
            url: url.to_string(),
            domain: domain.clone(),
            timestamp: 0, // Old timestamp so they are processed first
        };

        diesel::insert_into(queue::table)
            .values(new_element)
            .execute(db_conn)
            .unwrap();

        println!("[API] New URL added to the queue: {}", payload.url);
        return StatusCode::ACCEPTED;
    }

    StatusCode::BAD_REQUEST
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
    p: i32,
}

#[derive(utoipa::ToSchema, Serialize)]
pub struct ResultPageMetadata {
    title: Option<String>,
    description: Option<String>,
    theme_color: Option<String>,
    keywords: Option<String>,
    image: Option<String>,
}

#[derive(utoipa::ToSchema, Serialize)]
pub struct ResultPage {
    url: String,
    favicon: Option<String>,
    score: f32,
    clicks: i32,
    impressions: i32,
    likes: i32,
    dislikes: i32,
    crawled_at: i64,
    indexed_at: i64,
    metadata: ResultPageMetadata,
}

#[derive(utoipa::ToSchema, Serialize)]
pub struct SearchResponse {
    results: Vec<ResultPage>,
    time: i32,
    page: i32,
    total_pages: i32,
    total_results: i32,
}

#[utoipa::path(
    get,
    path = "/search",
    description = "Search the web",
    params(
        ("q" = String, Query, description = "The search query"),
        ("p" = String, Query, description = "The page")
    ),
    responses(
        (status = OK, body = SearchResponse)
    ),
)]
#[axum::debug_handler]
pub async fn get_search_handler(
    headers: HeaderMap,
    State(state): State<Arc<Environment>>,
    query: Query<SearchQuery>,
) -> Response {
    let user_query = query.q.trim().to_lowercase();
    if user_query.is_empty() || user_query.len() >= 256 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let page = query.p;
    if page < 1 || page > 100_000 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let start = Instant::now();
    let db_conn = &mut state.db_pool.get().unwrap();

    let search_results = search_pages(db_conn, user_query.clone());
    // let scores = tf_idf(db_conn, user_query.clone());

    let limit = 10usize;
    let offset_start = ((page as usize) - 1) * limit;
    let offset_end = offset_start + limit;
    let results_len = search_results.len();
    let paginated = &search_results[offset_start..offset_end.min(results_len)];
    let total_pages = results_len / limit;
    let time_taken = start.elapsed().as_nanos();

    let page_ids: Vec<i32> = paginated.iter().map(|x| x.0.id).collect();
    let mut result_pages = Vec::new();

    let analytics = pages_analytics::table
        .select(pages_analytics::all_columns)
        .filter(pages_analytics::page_id.eq_any(page_ids.clone()))
        .get_results::<PageAnalytics>(db_conn)
        .unwrap();
    let votes = get_vote_counts(db_conn, page_ids.clone()).unwrap();

    for (page, score) in paginated {
        // Should be valid
        let last_indexed = page.last_indexed.unwrap();

        let page_analytics = analytics.iter().find(|x| x.page_id == page.id);
        let page_votes = votes.iter().find(|x| x.page_id == page.id);

        result_pages.push(ResultPage {
            url: page.url.clone(),
            favicon: get_page_favicon(page.favicon_id),
            score: score.clone(),
            clicks: page_analytics.map(|x| x.clicks).unwrap_or(0),
            impressions: page_analytics.map(|x| x.impressions).unwrap_or(0),
            likes: page_votes.map(|x| x.like_count as i32).unwrap_or(0),
            dislikes: page_votes.map(|x| x.dislike_count as i32).unwrap_or(0),
            crawled_at: page.last_crawled,
            indexed_at: last_indexed,
            metadata: ResultPageMetadata {
                title: page.title.clone(),
                description: page.meta_description.clone(),
                theme_color: page.meta_theme_color.clone(),
                keywords: page.meta_keywords.clone(),
                image: page.meta_og_image.clone(),
            },
        });
    }

    // Analytics
    increment_impressions(db_conn, page_ids).unwrap();

    diesel::insert_into(queries::table)
        .values(NewQuery {
            query: user_query.clone(),
            timestamp: get_sql_timestamp(),
            search_time: time_taken as i32,
            result_count: results_len as i32,
            user_agent: headers
                .get(USER_AGENT)
                .map(|h| safe_slice(h.to_str().unwrap_or(""), 255).to_string()),
        })
        .execute(db_conn)
        .unwrap();

    // Response
    let search_response = SearchResponse {
        results: result_pages,
        time: time_taken as i32,
        page,
        total_pages: total_pages as i32,
        total_results: results_len as i32,
    };

    Json(search_response).into_response()
}

#[derive(QueryableByName)]
pub struct VoteCount {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub page_id: i32,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub like_count: i64,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub dislike_count: i64,
}

pub fn get_vote_counts(conn: &mut DbConn, ids: Vec<i32>) -> QueryResult<Vec<VoteCount>> {
    let ids_str = ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");

    if ids.is_empty() {
        return Ok(vec![]);
    }

    let query = format!(
        r#"
        SELECT 
            page_id,
            COUNT(*) FILTER (WHERE vote_type = 1) AS like_count,
            COUNT(*) FILTER (WHERE vote_type = 2) AS dislike_count
        FROM votes
        WHERE page_id IN ({})
        GROUP BY page_id
    "#,
        ids_str
    );

    sql_query(query).load::<VoteCount>(conn)
}

pub fn get_page_favicon(favicon_id: i32) -> Option<String> {
    let filename = format!("{}-", favicon_id);

    // TODO: use the const from favicons workspace
    let base_path = env::current_dir().unwrap().join("favicons");
    let paths = fs::read_dir(&base_path).unwrap();

    let mut encoded_favicon = None;

    for path in paths {
        let path = path.unwrap().file_name().into_string().unwrap();

        if path.starts_with(&filename) {
            let image_bytes = fs::read(&base_path.join(path)).unwrap();
            let base64_string = STANDARD.encode(image_bytes);
            encoded_favicon = Some(base64_string);
            break;
        }
    }

    encoded_favicon
}

pub fn increment_impressions(conn: &mut DbConn, page_ids: Vec<i32>) -> QueryResult<()> {
    if page_ids.is_empty() {
        return Ok(());
    }

    let new_rows: Vec<NewPageAnalytics> = page_ids
        .iter()
        .map(|&id| NewPageAnalytics {
            page_id: id,
            clicks: 0,
            impressions: 1,
        })
        .collect();

    diesel::insert_into(pages_analytics::table)
        .values(&new_rows)
        .on_conflict(pages_analytics::page_id)
        .do_update()
        .set(pages_analytics::impressions.eq(pages_analytics::impressions + 1))
        .execute(conn)?;

    Ok(())
}

fn search_pages(conn: &mut DbConn, query: String) -> Vec<(Page, f32)> {
    let words_vec: Vec<&str> = query.split_whitespace().collect();

    let mut filter: Box<dyn BoxableExpression<_, _, SqlType = diesel::sql_types::Bool>> =
        Box::new(pages::url.like(format!("%{}%", words_vec[0])));

    for w in &words_vec[1..] {
        filter = Box::new(filter.or(pages::url.like(format!("%{}%", w))));
    }
    let pages = pages::table
        .select(pages::all_columns)
        .filter(pages::last_indexed.is_not_null())
        .filter(filter)
        .load::<Page>(conn)
        .expect("Error loading pages");

    let mut results = Vec::new();

    for page in pages {
        let pathname = &page.url;
        let pathname_len = pathname.len() as f32;
        let domain_score = 100.0 * (1.0 + ((50.0 - pathname_len.min(50.0)) / 50.0).powf(2.0));

        let mut metadata_multiplier = 1.0;
        if page.title.is_some() {
            metadata_multiplier += 0.1;
        }
        if page.meta_description.is_some() {
            metadata_multiplier += 0.1;
        }
        if page.meta_og_image.is_some() {
            metadata_multiplier += 0.2;
        }
        if page.seo_score > 0 {
            metadata_multiplier += (page.seo_score as f32) / 100.0;
        }

        let bonus_score = if page.domain.contains(&query) {
            50.0
        } else {
            0.0
        };

        results.push((page, domain_score * metadata_multiplier + bonus_score))
    }

    results
}

/// TODO: implement
fn _tf_idf(conn: &mut DbConn, query: String) -> HashMap<i32, f64> {
    let words_vec: Vec<&str> = query.split_whitespace().collect();

    let mut filter: Box<dyn BoxableExpression<_, _, SqlType = diesel::sql_types::Bool>> =
        Box::new(words::word.like(format!("%{}%", words_vec[0])));

    for w in &words_vec[1..] {
        filter = Box::new(filter.or(words::word.like(format!("%{}%", w))));
    }
    let words: Vec<Word> = words::table
        .filter(filter)
        // distinct?
        .limit(10)
        .load(conn)
        .expect("Error loading words");

    let page_count: i64 = pages::table
        .count()
        .get_result(conn)
        .expect("Error counting pages");

    let words_list: Vec<String> = words.iter().map(|w| w.word.clone()).collect();

    let result = indexes::table
        .inner_join(words::table.on(indexes::word_id.eq(words::id)))
        .inner_join(pages::table.on(indexes::page_id.eq(pages::id)))
        .filter(words::word.eq_any(&words_list))
        .select((
            pages::id,
            pages::url,
            words::word,
            indexes::count,
            sql::<diesel::sql_types::BigInt>(
                "(SELECT COUNT(DISTINCT page_id) FROM indexes WHERE word_id = indexes.word_id)",
            ),
        ))
        .limit(500)
        .load::<(i32, String, String, i32, i64)>(conn)
        .unwrap();

    println!("tf_idf RESULT: {:#?}", result);

    let mut tf_idf_scores = HashMap::new();

    for (page_id, url, _word, count, doc_count) in result {
        let tf = count as f64;
        let idf = ((page_count + 1) as f64 / (doc_count + 1) as f64).ln() + 1.0;
        println!("{url}: {count},{page_count},{doc_count}");

        *tf_idf_scores.entry(page_id).or_insert(0.0) += tf * idf;
    }

    println!("td_idf SCORES: {:#?}", tf_idf_scores);

    tf_idf_scores
}
