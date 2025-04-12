use crate::{downloader::Downloader, utils::get_favicons_directory};
use database::{models::Favicon, schema::favicons, DbPool};
use diesel::{query_dsl::QueryDsl, RunQueryDsl};
use std::{
    collections::HashMap,
    fs::{self},
    io::{self, ErrorKind},
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::Mutex;
use utils::url::normalize_url;

/// Manage the download of the pages favicons
pub struct Favicons {
    db_pool: DbPool,
    parallel_tasks: usize,
    downloader: Arc<Downloader>,
    favicon_directory: PathBuf,
}

impl Favicons {
    pub fn new(db_pool: DbPool, parallel_tasks: usize, user_agent: String) -> Self {
        Self {
            db_pool,
            parallel_tasks,
            downloader: Arc::new(Downloader::new(user_agent)),
            favicon_directory: get_favicons_directory(),
        }
    }

    pub async fn download_missing_favicons(&self) -> usize {
        let favicons_map = self.find_favicons_to_download();

        let count: usize = favicons_map.values().map(|v| v.len()).sum();
        println!(
            "Downloading {count} favicons with {} task(s)...",
            self.parallel_tasks
        );

        let favicons_map = Arc::new(Mutex::new(favicons_map));
        let mut tasks = Vec::new();

        for _ in 0..self.parallel_tasks {
            let map = favicons_map.clone();
            let downloader = self.downloader.clone();

            let t = tokio::spawn(async move {
                loop {
                    // Get a key (and remove it) from favicons_map
                    let favicons = {
                        let mut guard = map.lock().await;
                        let last_key = guard.keys().last().cloned();

                        if let Some(key) = last_key {
                            guard.remove(&key.clone())
                        } else {
                            None
                        }
                    };

                    if let Some(favicons) = favicons {
                        downloader.download_domain_favicons(favicons).await;
                    } else {
                        break;
                    }
                }
            });

            tasks.push(t);
        }

        for t in tasks {
            t.await.expect("A favicon task panicked");
        }

        println!("Favicons download ended");
        count
    }

    /// Determines the favicons that are missing from the favicons directory
    ///
    /// Returns HashMap<domain, Vec<(favicon_id, favicon_url)>>
    fn find_favicons_to_download(&self) -> HashMap<String, Vec<(i32, String)>> {
        let db_favicons = self.get_db_favicons_list();
        let downloaded_favicons = self
            .get_downloaded_favicons_list()
            .expect("Failed to get the downloaded favicons list");

        let mut missing_favicons = HashMap::new();

        for fav in db_favicons {
            if let Some((favicon_url, domain)) = normalize_url(&fav.url) {
                if let Some(_) = downloaded_favicons.get(&fav.id) {
                    // favicon already downloaded, continue
                    continue;
                } else {
                    // mark the favicon as missing
                    missing_favicons
                        .entry(domain)
                        .or_insert(Vec::new())
                        .push((fav.id, favicon_url.to_string()));
                }
            }
        }

        missing_favicons
    }

    /// Get the files list of the downloaded favicons and their download timestamp
    ///  
    /// Returns HashMap<page_id, favicon_download_timestamp>
    fn get_downloaded_favicons_list(&self) -> Result<HashMap<i32, i64>, io::Error> {
        if let Err(e) = fs::create_dir(&self.favicon_directory) {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create favicons directory: {e}");
            }
        }

        let paths = fs::read_dir(&self.favicon_directory)?;

        let mut favicons: HashMap<i32, i64> = HashMap::new();
        for path in paths {
            let file_name = path.unwrap().file_name().into_string().unwrap();

            if let Some((id, timestamp)) = file_name.split_once('-') {
                favicons.insert(id.parse().unwrap(), timestamp[..4].parse().unwrap());
            } else {
                eprintln!("The favicon file '{file_name}' is not in the correct format");
            }
        }

        Ok(favicons)
    }

    /// Get the crawled favicons URLs
    fn get_db_favicons_list(&self) -> Vec<Favicon> {
        let conn = &mut self.db_pool.get().unwrap();

        let results = favicons::table
            .select((favicons::id, favicons::url))
            .load::<Favicon>(conn)
            .unwrap();

        results
    }
}
