use database::DbPool;
use database::{
    models::Page,
    schema::{indexes, pages, words},
};
use diesel::{dsl::sql, upsert::excluded, ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel::{BoolExpressionMethods, NullableExpressionMethods};
use std::collections::HashMap;
use utils::sql::get_sql_timestamp;

pub const INDEXING_BATCH_SIZE: i64 = 1000;

/// When a page is indexed, all words indexes are created in one db call.
/// To not exceed the PostgreSQL limit, this is the max.
///
/// TODO: make multiple requests to always index all words
pub const MAX_WORD_COUNT: usize = (1 << 16) - 1;

/// TODO: should we add multi-threading?
pub struct Indexer {
    db_pool: DbPool,
}

impl Indexer {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }

    /// Get pages to index
    async fn get_pages(&self) -> Vec<Page> {
        let results = pages::table
            .select(pages::all_columns)
            .filter(
                pages::last_indexed
                    .is_null()
                    .or(pages::last_crawled.nullable().gt(pages::last_indexed)),
            )
            .limit(INDEXING_BATCH_SIZE)
            .load::<Page>(&mut self.db_pool.get().unwrap())
            .unwrap();

        results
    }

    /// Start indexing all pages
    pub async fn index(&self) -> usize {
        let pages = self.get_pages().await;
        let len = pages.len();

        println!("Indexing {len} pages...");

        for page in pages {
            self.index_page(page).await;
        }

        println!("Indexed {len} pages");

        len
    }

    async fn index_page(&self, page: Page) {
        let db_conn = &mut self.db_pool.get().unwrap();

        // Index the words
        if let Some(content) = page.content {
            let words_count = self.tokenize(&content);
            let words_list: Vec<String> = words_count.keys().cloned().collect();

            if words_count.len() > 0 && words_count.len() < MAX_WORD_COUNT {
                // Insert the new words (if some) and return them
                let inserted_words: Vec<(i32, String)> = diesel::insert_into(words::table)
                    .values(
                        words_list
                            .iter()
                            .map(|w| words::word.eq(w))
                            .collect::<Vec<_>>(),
                    )
                    .on_conflict(words::word)
                    .do_update()
                    .set(words::word.eq(excluded(words::word)))
                    .returning((words::id, words::word))
                    .load(db_conn)
                    .unwrap();

                // Update the indexes

                let word_ids: HashMap<String, i32> = inserted_words
                    .into_iter()
                    .map(|(id, word)| (word, id))
                    .collect();

                let new_indexes: Vec<_> = words_count
                    .into_iter()
                    .map(|(word, count)| {
                        let word_id = *word_ids.get(&word).unwrap();
                        (
                            indexes::word_id.eq(word_id),
                            indexes::page_id.eq(page.id),
                            indexes::count.eq(count),
                        )
                    })
                    .collect();

                // Insert the new indexes
                diesel::insert_into(indexes::table)
                    .values(new_indexes)
                    .on_conflict((indexes::word_id, indexes::page_id))
                    .do_update()
                    .set(indexes::count.eq(sql("excluded.count")))
                    .execute(db_conn)
                    .unwrap();
            }
        }

        // Mark the table as indexed
        diesel::update(pages::table)
            .filter(pages::id.eq(page.id))
            .set(pages::last_indexed.eq(get_sql_timestamp()))
            .execute(db_conn)
            .unwrap();
    }

    /// Divides the content into lowercase words
    /// A word length is `>= 1 && <= 100`
    ///
    /// Returns HashMap<word, count>
    fn tokenize(&self, content: &str) -> HashMap<String, i32> {
        let mut word_count = HashMap::new();

        for word in content.split_whitespace() {
            let clean_word = word
                .to_lowercase()
                .trim_matches(|c: char| !c.is_alphabetic())
                .to_string();

            if !clean_word.is_empty() && clean_word.len() <= 100 {
                *word_count.entry(clean_word.to_string()).or_insert(0) += 1;
            }
        }

        word_count
    }
}
