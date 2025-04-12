// @generated automatically by Diesel CLI.

diesel::table! {
    favicons (id) {
        id -> Int4,
        #[max_length = 2048]
        url -> Varchar,
    }
}

diesel::table! {
    indexes (word_id, page_id) {
        word_id -> Int4,
        page_id -> Int4,
        count -> Int4,
    }
}

diesel::table! {
    links (id) {
        id -> Int4,
        from_page_id -> Int4,
        to_page_id -> Int4,
    }
}

diesel::table! {
    pages (id) {
        id -> Int4,
        #[max_length = 100]
        domain -> Varchar,
        #[max_length = 2048]
        url -> Varchar,
        #[max_length = 100]
        title -> Nullable<Varchar>,
        favicon_id -> Int4,
        #[max_length = 65535]
        content -> Nullable<Varchar>,
        #[max_length = 65535]
        body -> Nullable<Varchar>,
        body_length -> Int4,
        #[max_length = 100]
        content_type -> Varchar,
        response_time -> Int4,
        status_code -> Int4,
        last_crawled -> Int8,
        last_indexed -> Nullable<Int8>,
        seo_score -> Int4,
        #[max_length = 200]
        meta_description -> Nullable<Varchar>,
        #[max_length = 200]
        meta_keywords -> Nullable<Varchar>,
        #[max_length = 6]
        meta_theme_color -> Nullable<Varchar>,
        #[max_length = 512]
        meta_og_image -> Nullable<Varchar>,
    }
}

diesel::table! {
    pages_analytics (id) {
        id -> Int4,
        page_id -> Int4,
        clicks -> Int4,
        impressions -> Int4,
    }
}

diesel::table! {
    queries (id) {
        id -> Int4,
        #[max_length = 512]
        query -> Varchar,
        timestamp -> Int8,
        search_time -> Int4,
        result_count -> Int4,
        #[max_length = 255]
        user_agent -> Nullable<Varchar>,
    }
}

diesel::table! {
    queue (id) {
        id -> Int4,
        #[max_length = 100]
        domain -> Varchar,
        #[max_length = 2048]
        url -> Varchar,
        timestamp -> Int8,
    }
}

diesel::table! {
    statistics (id) {
        id -> Int4,
        statistic_type -> Int4,
        value -> Int8,
        timestamp -> Int8,
    }
}

diesel::table! {
    votes (id) {
        id -> Int4,
        page_id -> Int4,
        #[max_length = 100]
        ip -> Varchar,
        #[max_length = 100]
        fingerprint -> Varchar,
        vote_type -> Int4,
        updated_at -> Int8,
        created_at -> Int8,
    }
}

diesel::table! {
    words (id) {
        id -> Int4,
        #[max_length = 100]
        word -> Varchar,
    }
}

diesel::joinable!(indexes -> pages (page_id));
diesel::joinable!(indexes -> words (word_id));
diesel::joinable!(pages -> favicons (favicon_id));
diesel::joinable!(pages_analytics -> pages (page_id));
diesel::joinable!(votes -> pages (page_id));

diesel::allow_tables_to_appear_in_same_query!(
    favicons,
    indexes,
    links,
    pages,
    pages_analytics,
    queries,
    queue,
    statistics,
    votes,
    words,
);
