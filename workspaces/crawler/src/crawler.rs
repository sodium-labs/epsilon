use crate::utils::is_crawlable_url;
use crate::website::Website;
use crate::worker::Worker;
use dashmap::{DashMap, DashSet};
use database::models::QueuedPage;
use database::schema::pages;
use database::DbPool;
use diesel::query_dsl::methods::SelectDsl;
use diesel::RunQueryDsl;
use reqwest::redirect::Policy;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::sleep;
use utils::url::normalize_url;

pub const DEFAULT_LOCAL_QUEUE_SIZE: usize = 1000;

#[derive(Clone)]
pub struct Task {
    pub id: i32,
    pub domain: String,
    pub url: String,
}

pub struct Crawler {
    pub user_agent: String,
    pub web_client: Client,
    pub db_pool: DbPool,

    pub visited: DashSet<String>,
    pub websites: DashMap<String, Website>,
    pub queue_channel: (Sender<Task>, Mutex<Receiver<Task>>),
}

impl Crawler {
    pub fn new(db_pool: DbPool, user_agent: String, local_queue_size: Option<usize>) -> Self {
        let local_queue_size = local_queue_size.unwrap_or(DEFAULT_LOCAL_QUEUE_SIZE);
        let queue = channel(local_queue_size);
        println!("Crawler local queue size: {local_queue_size}");

        let urls = Crawler::load_visited_urls(&db_pool);
        let client = Client::builder()
            .user_agent(&user_agent)
            .timeout(Duration::from_secs(10))
            .redirect(Policy::default())
            .build()
            .unwrap();

        Self {
            user_agent,
            web_client: client,
            db_pool,
            visited: urls,
            websites: DashMap::new(),
            queue_channel: (queue.0, Mutex::new(queue.1)),
        }
    }

    fn load_visited_urls(db_pool: &DbPool) -> DashSet<String> {
        let results = pages::table
            .select(pages::url)
            .load::<String>(&mut db_pool.get().unwrap())
            .expect("Failed to load URLs");

        let visited_urls: DashSet<String> = results.into_iter().collect();

        visited_urls
    }

    pub fn get_crawled_pages_count(&self) -> i64 {
        use diesel::QueryDsl;

        let count = pages::table
            .count()
            .get_result(&mut self.db_pool.get().unwrap())
            .expect("Failed to count pages");

        count
    }

    pub async fn start_crawling(&self, arc: Arc<Crawler>, threads: usize) {
        println!("Starting crawling with {threads} threads");

        let mut tasks = Vec::new();
        self.fill_queue(arc.clone());

        for _ in 0..threads {
            let manager = arc.clone();

            let handle = task::spawn(async {
                let mut worker = Worker::new(manager);
                worker.crawl().await;
            });

            tasks.push(handle);
        }

        task::spawn({
            let manager = arc.clone();
            async move {
                let mut count = 0;
                let delay = Duration::from_millis(1500);
                loop {
                    sleep(delay).await;

                    let new_count = { manager.get_crawled_pages_count() };

                    let per_sec = (new_count - count) as f32 / delay.as_secs_f32();
                    let old_count = count;
                    count = new_count;

                    if old_count != 0 {
                        println!("\r- [Crawler] {per_sec:.2} pages/s --- ({count})");
                        // std::io::stdout().flush().unwrap();
                    }
                }
            }
        });

        for task in tasks {
            task.await
                .unwrap_or_else(|_| panic!("Crawler task panicked!"));
        }
        println!("Crawling finished");
    }

    fn fill_queue(&self, arc: Arc<Crawler>) {
        let tx_clone = self.queue_channel.0.clone();

        tokio::spawn(async move {
            loop {
                let p = &arc.clone().db_pool;
                let tasks = Crawler::dequeue(p).await;
                if tasks.is_empty() {
                    sleep(Duration::from_secs(1)).await;
                } else {
                    for task in tasks {
                        if let Some((url, domain)) = normalize_url(&task.url) {
                            if !is_crawlable_url(&url.to_string()) {
                                continue;
                            }

                            let task = Task {
                                id: task.id,
                                domain,
                                url: url.to_string(),
                            };

                            if tx_clone.send(task).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
    async fn dequeue(db_pool: &DbPool) -> Vec<QueuedPage> {
        // println!("Dequeue-ing from the DB");

        let elements: Vec<QueuedPage> = diesel::sql_query(
            "
WITH recent_domains AS (
    SELECT domain
    FROM queue
    GROUP BY domain
    ORDER BY MAX(timestamp) DESC
    LIMIT 10
),
selected AS (
    SELECT id, url, timestamp
    FROM queue
    WHERE domain IN (SELECT domain FROM recent_domains)
    ORDER BY timestamp DESC
    LIMIT 100
)
DELETE FROM queue
WHERE id IN (SELECT id FROM selected)
RETURNING id, domain, url, timestamp;
        ",
        )
        .load::<QueuedPage>(&mut db_pool.get().unwrap())
        .unwrap();
        elements
    }
}
