CREATE TABLE queue (
    id SERIAL PRIMARY KEY,
    domain VARCHAR(100) NOT NULL,
    url VARCHAR(2048) UNIQUE NOT NULL,
    timestamp BIGINT NOT NULL
);

CREATE TABLE favicons (
    id SERIAL PRIMARY KEY,
    url VARCHAR(2048) UNIQUE NOT NULL
);

CREATE TABLE pages (
    id SERIAL PRIMARY KEY,
    domain VARCHAR(100) NOT NULL,
    url VARCHAR(2048) UNIQUE NOT NULL,
    title VARCHAR(100),
    favicon_id INT NOT NULL REFERENCES favicons(id) ON DELETE CASCADE,
    content VARCHAR(65535),
    body VARCHAR(65535),
    body_length INT NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    response_time INT NOT NULL,
    status_code INT NOT NULL,
    last_crawled BIGINT NOT NULL,
    last_indexed BIGINT,
    seo_score INT NOT NULL,
    meta_description VARCHAR(200),
    meta_keywords VARCHAR(200),
    meta_theme_color VARCHAR(6),
    meta_og_image VARCHAR(512)
);

CREATE TABLE pages_analytics (
    id SERIAL PRIMARY KEY,
    page_id INT UNIQUE NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    clicks INT NOT NULL,
    impressions INT NOT NULL
);

CREATE TABLE queries (
    id SERIAL PRIMARY KEY,
    query VARCHAR(512) NOT NULL,
    timestamp BIGINT NOT NULL,
    search_time INT NOT NULL,
    result_count INT NOT NULL,
    user_agent VARCHAR(255)
);

CREATE TABLE statistics (
    id SERIAL PRIMARY KEY,
    statistic_type INT NOT NULL,
    value BIGINT NOT NULL,
    timestamp BIGINT NOT NULL
);

CREATE TABLE votes (
    id SERIAL PRIMARY KEY,
    page_id INT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    ip VARCHAR(100) NOT NULL,
    fingerprint VARCHAR(100) NOT NULL,
    vote_type INT NOT NULL,
    updated_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    CONSTRAINT unique_vote UNIQUE (page_id, fingerprint)
);

CREATE TABLE links (
    id SERIAL PRIMARY KEY,
    from_page_id INT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    to_page_id INT NOT NULL REFERENCES pages(id) ON DELETE CASCADE
);

CREATE TABLE words (
    id SERIAL PRIMARY KEY,
    word VARCHAR(100) UNIQUE NOT NULL
);

CREATE TABLE indexes (
    word_id INT NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    page_id INT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    count INT NOT NULL,
    PRIMARY KEY (word_id, page_id)
);

CREATE INDEX idx_pages_url ON pages(url);
CREATE INDEX idx_index_word ON indexes(word_id);
CREATE INDEX idx_index_page ON indexes(page_id);
CREATE INDEX idx_pages_analytics_page ON pages_analytics(page_id);
CREATE INDEX idx_words_word ON words(word);