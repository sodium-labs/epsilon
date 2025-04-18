use crate::crawler::Crawler;
use crate::utils::{calculate_seo_score, get_content_type};
use crate::website::Website;
use crate::{crawler::Task, scraper::scrape_page};
use dashmap::mapref::one::RefMut;
use database::models::{NewFavicon, NewPage, NewQueuedPage};
use database::schema::{favicons, pages, queue};
use diesel::prelude::*;
use std::{
    collections::HashSet,
    sync::Arc,
    time::Instant,
};
use url::Url;
use utils::safe_slice;
use utils::sql::get_sql_timestamp;
use utils::url::normalize_url;

pub const DOMAIN_CRAWL_COOLDOWN: u128 = 10_000;

// CrawlError //

#[derive(Debug)]
enum CrawlError {
    ServerError,
    InvalidContentType,
    Reqwest(reqwest::Error),
    NotCrawlable,
    Redirect(String, Url),
    ParseError,
}

impl From<reqwest::Error> for CrawlError {
    fn from(value: reqwest::Error) -> Self {
        CrawlError::Reqwest(value)
    }
}

// Worker //

pub struct Worker {
    manager: Arc<Crawler>,
}

impl Worker {
    pub fn new(manager: Arc<Crawler>) -> Self {
        Self { manager }
    }

    async fn dequeue(&mut self) -> Option<Task> {
        let mut rx = self.manager.queue_channel.1.lock().await;
        rx.recv().await
    }

    fn get_website(&self, domain: String) -> RefMut<'_, String, Website> {
        let website = self
            .manager
            .websites
            .entry(domain.clone())
            .or_insert(Website::new(domain));
        website
    }

    async fn can_crawl(&self, task: Task) -> bool {
        let should_fetch_robots = {
            let website = self.get_website(task.domain.clone());
            website.should_fetch_robots()
        };
        // The website lock is dropped before the potential await

        let mut website;
        if should_fetch_robots {
            let robots = Website::fetch_robots(task.domain.clone(), &self.manager.web_client).await;

            website = self.get_website(task.domain.clone());
            if robots.is_ok() {
                website.set_robots(robots.unwrap());
            }
        } else {
            website = self.get_website(task.domain.clone());
        }

        if !website.is_crawlable(&self.manager.user_agent, &task.url) {
            return false;
        }

        // Rate limits
        if let Some(last_crawl) = &website.last_crawl {
            let elapsed = last_crawl.elapsed().as_millis();

            if elapsed < DOMAIN_CRAWL_COOLDOWN {
                // println!("cooldown: {} / {}", task.url.clone(), website.domain);

                // Drop the website as soon as possible to drop the lock
                drop(website);

                // let delay = DOMAIN_CRAWL_COOLDOWN - elapsed;

                // This domain cannot be crawled for now, send it back in the queue
                // TODO: currently this push the url to the back of the queue, fix that
                self.save_to_queue(task.domain, task.url);
                return false;
            }
        }

        website.last_crawl = Some(Instant::now());
        true
    }

    pub async fn crawl(&mut self) {
        while let Some(task) = self.dequeue().await {
            if self.manager.visited.contains(&task.url) {
                continue;
            }

            if !self.can_crawl(task.clone()).await {
                continue;
            }

            self.manager.visited.insert(task.url.clone());

            match self.crawl_page(&task).await {
                Ok((page, favicon, links)) => {
                    let mut new_links = HashSet::new();

                    for l in links {
                        if let Some((url, domain)) = normalize_url(&l) {
                            let stringified_url = url.to_string();
                            if self.manager.visited.contains(&stringified_url) {
                                continue;
                            }
                            new_links.insert((domain, stringified_url));
                        }
                    }

                    self.save_page(page, favicon, new_links);
                }
                Err(CrawlError::Reqwest(e)) => {
                    if e.is_timeout() {
                        self.save_to_queue(task.domain, task.url);
                        continue;
                    } else if e.is_redirect() {
                        continue;
                    }
                    if e.is_connect() {
                        continue;
                    }
                    if e.is_request() {
                        continue;
                        // TODO: errors should be logged in a file, not in the console.
                        /*if let Some(std_error) = e.source() {
                            let error_string = format!("{std_error}");
                            // unknown host
                            if error_string.contains("Os { code: 11001, ") {
                                continue;
                            }
                            // connection reset
                            if error_string.contains("Os { code: 10054, ") {
                                continue;
                            }
                            // invalid certificate error
                            if error_string.contains("Os { code: -2146762481, ") {
                                continue;
                            }
                            // invalid certificate error
                            if error_string.contains("Os { code: -2146762495, ") {
                                continue;
                            }
                        }*/
                    }
                    eprintln!("reqwest error when crawling {}: {:?}", task.url, e);
                }
                Err(CrawlError::ParseError) | Err(CrawlError::ServerError) => {
                    self.save_to_queue(task.domain, task.url);
                }
                Err(CrawlError::Redirect(domain, url)) => {
                    if self.manager.visited.contains(&url.to_string()) {
                        continue;
                    }
                    self.save_to_queue(domain, url.to_string());
                }
                Err(CrawlError::NotCrawlable) => {
                    // Ignore
                }
                Err(e) => {
                    eprintln!("Error when crawling {}: {:?}", task.url, e);
                }
            }
        }
    }

    /// Crawl a page and returns the links present on the page
    async fn crawl_page(
        &self,
        task: &Task,
    ) -> Result<(NewPage, NewFavicon, HashSet<String>), CrawlError> {
        // println!("Crawling {}", &task.url);

        let start_at = Instant::now();
        let response = self.manager.web_client.get(task.url.clone()).send().await?;

        let response_time = (Instant::now() - start_at).as_millis().try_into().unwrap();
        let status_code = response.status();

        if status_code.is_server_error() {
            return Err(CrawlError::ServerError);
        }

        if !status_code.is_success() {
            return Err(CrawlError::NotCrawlable);
        }

        if let Some((new_url, domain)) = normalize_url(&response.url().to_string()) {
            if new_url.to_string() != task.url.to_string() {
                // The URL changed, so the task infos are invalid
                return Err(CrawlError::Redirect(domain, new_url));
            }
        } else {
            return Err(CrawlError::NotCrawlable);
        }

        let headers = response.headers();
        let content_type = get_content_type(headers, &task.url);

        if let Some(content_type) = content_type {
            if content_type != "text/html" {
                return Err(CrawlError::InvalidContentType);
            }
        }

        let text_result = response.text().await?;

        match scrape_page(task.domain.clone(), task.url.clone(), text_result) {
            Ok(mut scraped) => {
                let seo_score = calculate_seo_score(&scraped);

                let page = NewPage {
                    domain: task.domain.clone(),
                    url: task.url.clone(),
                    title: scraped.title.map(|x| safe_slice(&x, 100).to_string()),
                    favicon_id: -1,
                    content: scraped.content,
                    body: scraped.html, // TODO: Length check
                    body_length: scraped.html_length.try_into().unwrap(),
                    content_type: "text/html".into(),
                    response_time,
                    status_code: status_code.as_u16().into(),
                    last_crawled: get_sql_timestamp(),
                    last_indexed: None,
                    seo_score,
                    meta_description: scraped
                        .meta_description
                        .map(|x| safe_slice(&x, 200).to_string()),
                    meta_keywords: scraped
                        .meta_keywords
                        .map(|x| safe_slice(&x, 200).to_string()),
                    meta_theme_color: scraped.meta_theme_color.map(|x| {
                        let v = if x.starts_with("#") {
                            &x[1..]
                        } else {
                            x.as_str()
                        };
                        safe_slice(v, 6).to_string()
                    }),
                    meta_og_image: scraped
                        .meta_og_image
                        .map(|x| safe_slice(&x, 512).to_string()),
                };

                let favicon = NewFavicon {
                    url: scraped
                        .favicon_url
                        .take_if(|x| x.len() <= 2048)
                        .unwrap_or(format!("https://{}/favicon.ico", task.domain)),
                };

                Ok((page, favicon, scraped.links))
            }
            Err(e) => {
                eprintln!("Failed to scrape page: {e:?}");
                Err(CrawlError::ParseError)
            }
        }
    }

    /// Save the collected page data
    fn save_page(&self, mut page: NewPage, favicon: NewFavicon, links: HashSet<(String, String)>) {
        let db_conn = &mut self.manager.db_pool.get().unwrap();

        let favicon_url = favicon.url.clone();

        // Insert the new favicon
        let favicon_id = diesel::insert_into(favicons::table)
            .values(favicon)
            .on_conflict(favicons::url)
            .do_update()
            .set(favicons::url.eq(favicon_url))
            .returning(favicons::id)
            .get_result::<i32>(db_conn)
            .unwrap();

        page.favicon_id = favicon_id;

        // Insert the page
        diesel::insert_into(pages::table)
            .values(page)
            .execute(db_conn)
            .unwrap();

        let elements = links
            .iter()
            .filter(|x| x.1.len() <= 2048)
            .map(|x| NewQueuedPage {
                url: x.1.clone(),
                domain: x.0.clone(),
                timestamp: get_sql_timestamp(),
            })
            .collect::<Vec<_>>();

        // Insert the new urls to the queue
        diesel::insert_into(queue::table)
            .values(elements)
            .on_conflict(queue::url)
            .do_nothing()
            .execute(db_conn)
            .unwrap();
    }

    /// Put back a URL in the database queue
    fn save_to_queue(&self, domain: String, url: String) {
        // Remove it from the visited so it can be crawled again
        self.manager.visited.remove(&url);

        let db_conn = &mut self.manager.db_pool.get().unwrap();

        diesel::insert_into(queue::table)
            .values(NewQueuedPage {
                domain,
                url,
                timestamp: get_sql_timestamp(),
            })
            .on_conflict(queue::url)
            .do_nothing()
            .execute(db_conn)
            .unwrap();
    }
}