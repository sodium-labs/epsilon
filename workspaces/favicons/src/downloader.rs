use crate::utils::get_favicons_directory;
use image::{imageops::FilterType, ImageError, ImageFormat};
use reqwest::Client;
use std::{
    fs::File,
    io::{self, BufWriter},
    path::PathBuf,
    time::Duration,
};
use tokio::time::sleep;
use utils::get_timestamp;

pub const DOMAIN_COOLDOWN: u64 = 10_000;

pub const FAVICON_SIZE: u32 = 32;

#[derive(Debug)]
enum FaviconDownloadError {
    Reqwest(reqwest::Error),
    Image(ImageError),
    File(io::Error),
}

impl From<reqwest::Error> for FaviconDownloadError {
    fn from(value: reqwest::Error) -> Self {
        FaviconDownloadError::Reqwest(value)
    }
}

impl From<ImageError> for FaviconDownloadError {
    fn from(value: ImageError) -> Self {
        FaviconDownloadError::Image(value)
    }
}

impl From<io::Error> for FaviconDownloadError {
    fn from(value: io::Error) -> Self {
        FaviconDownloadError::File(value)
    }
}

pub struct Downloader {
    client: Client,
    favicon_directory: PathBuf,
}

impl Downloader {
    pub fn new(user_agent: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent(user_agent)
                .build()
                .expect("Failed to build the reqwest Client"),
            favicon_directory: get_favicons_directory(),
        }
    }

    /// Download all favicons from a single domain
    pub async fn download_domain_favicons(&self, favicons: Vec<(i32, String)>) {
        let len = favicons.len();
        let mut i = 0;

        for (fav_id, fav_url) in favicons {
            if let Err(e) = self.download_favicon(fav_id, fav_url.clone()).await {
                match e {
                    FaviconDownloadError::Reqwest(err) => {
                        let _ = err; // temp remove the warn
                                     // eprintln!("Failed to download favicon {fav_id}: {err:?}");
                    }
                    FaviconDownloadError::Image(err) => {
                        let _ = err; // temp remove the warn
                                     // eprintln!("Failed to create the image of favicon {fav_id}: {err:?}");
                    }
                    FaviconDownloadError::File(err) => {
                        eprintln!("Failed to write file of favicon {fav_id}: {err:?}");
                    }
                };
            }

            if (i + 1) < len {
                sleep(Duration::from_millis(DOMAIN_COOLDOWN)).await;
            }

            i += 1;
        }
    }

    async fn download_favicon(
        &self,
        fav_id: i32,
        fav_url: String,
    ) -> Result<(), FaviconDownloadError> {
        let response = self.client.get(fav_url).send().await?;
        let bytes = response.bytes().await?;

        let img = image::load_from_memory(&bytes)?;
        let resized = img.resize_exact(FAVICON_SIZE, FAVICON_SIZE, FilterType::Lanczos3);

        let now = get_timestamp().as_millis().to_string();
        let output_path = format!("{}-{}.png", fav_id, now);

        let file = File::create(&self.favicon_directory.join(output_path))?;
        let writer = &mut BufWriter::new(file);

        resized.write_to(writer, ImageFormat::Png)?;
        Ok(())
    }
}
