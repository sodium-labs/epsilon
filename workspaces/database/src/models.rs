use crate::types::{StatisticType, VoteType};
use diesel::prelude::*;

// Queue //

#[derive(QueryableByName, Queryable, Selectable)]
#[diesel(table_name = crate::schema::queue)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct QueuedPage {
    pub id: i32,
    pub domain: String,
    pub url: String,
    pub timestamp: i64,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::queue)]
pub struct NewQueuedPage {
    pub domain: String,
    pub url: String,
    pub timestamp: i64,
}

// Links //

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::links)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Link {
    pub id: i32,
    pub from_page_id: i32,
    pub to_page_id: i32,
}

// Favicons //

#[derive(Insertable)]
#[diesel(table_name = crate::schema::favicons)]
pub struct NewFavicon {
    pub url: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::favicons)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Favicon {
    pub id: i32,
    pub url: String,
}

// Pages //

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::pages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Page {
    pub id: i32,
    pub domain: String,
    pub url: String,
    pub title: Option<String>,
    pub favicon_id: i32,
    pub content: Option<String>,
    pub body: Option<String>,
    pub body_length: i32,
    pub content_type: String,
    pub response_time: i32,
    pub status_code: i32,
    pub last_crawled: i64,
    pub last_indexed: Option<i64>,
    pub seo_score: i32,
    pub meta_description: Option<String>,
    pub meta_keywords: Option<String>,
    pub meta_theme_color: Option<String>,
    pub meta_og_image: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::pages)]
pub struct NewPage {
    pub domain: String,
    pub url: String,
    pub title: Option<String>,
    pub favicon_id: i32,
    pub content: Option<String>,
    pub body: Option<String>,
    pub body_length: i32,
    pub content_type: String,
    pub response_time: i32,
    pub status_code: i32,
    pub last_crawled: i64,
    pub last_indexed: Option<i64>,
    pub seo_score: i32,
    pub meta_description: Option<String>,
    pub meta_keywords: Option<String>,
    pub meta_theme_color: Option<String>,
    pub meta_og_image: Option<String>,
}

// Pages Analytics //

#[derive(Insertable)]
#[diesel(table_name = crate::schema::pages_analytics)]
pub struct NewPageAnalytics {
    pub page_id: i32,
    pub clicks: i32,
    pub impressions: i32,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::pages_analytics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PageAnalytics {
    pub id: i32,
    pub page_id: i32,
    pub clicks: i32,
    pub impressions: i32,
}

// Queries //

#[derive(Insertable)]
#[diesel(table_name = crate::schema::queries)]
pub struct NewQuery {
    pub query: String,
    pub timestamp: i64,
    pub search_time: i32,
    pub result_count: i32,
    pub user_agent: Option<String>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::queries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Query {
    pub id: i32,
    pub query: String,
    pub timestamp: i64,
    pub search_time: i32,
    pub result_count: i32,
    pub user_agent: Option<String>,
}

// Votes //

#[derive(Insertable)]
#[diesel(table_name = crate::schema::votes)]
pub struct NewVote {
    pub page_id: i32,
    pub ip: String,
    pub fingerprint: String,
    pub vote_type: VoteType,
    pub updated_at: i64,
    pub created_at: i64,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::votes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Vote {
    pub id: i32,
    pub page_id: i32,
    pub ip: String,
    pub fingerprint: String,
    pub vote_type: VoteType,
    pub updated_at: i64,
    pub created_at: i64,
}

// Indexes //

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::indexes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Index {
    pub word_id: i32,
    pub page_id: i32,
    pub count: i32,
}

// Words //

#[derive(Insertable)]
#[diesel(table_name = crate::schema::words)]
pub struct NewWord {
    pub word: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::words)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Word {
    pub id: i32,
    pub word: String,
}

// Statistics //

#[derive(Insertable)]
#[diesel(table_name = crate::schema::statistics)]
pub struct NewStatistic {
    pub statistic_type: StatisticType,
    pub value: i64,
    pub timestamp: i64,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::statistics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Statistic {
    pub id: i32,
    pub statistic_type: StatisticType,
    pub value: i64,
    pub timestamp: i64,
}
