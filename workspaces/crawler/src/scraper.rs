use crate::utils::extract_words;
use scraper::{Html, Selector};
use std::{collections::HashSet, error::Error};
use utils::{safe_slice, url::normalize_href};

const LINK_SELECTOR: &str = concat!(
    "a[href]",
    ":not([href$=\".jpg\"])",
    ":not([href$=\".jpeg\"])",
    ":not([href$=\".png\"])",
    ":not([href$=\".gif\"])",
    ":not([href$=\".svg\"])",
    ":not([href$=\".webp\"])",
    ":not([href$=\".mp4\"])",
    ":not([href$=\".avi\"])",
    ":not([href$=\".mov\"])",
    ":not([href$=\".wmv\"])",
    ":not([href$=\".flv\"])",
    ":not([href$=\".mp3\"])",
    ":not([href$=\".wav\"])",
    ":not([href$=\".wma\"])",
    ":not([href$=\".wpl\"])",
    ":not([href$=\".mpa\"])",
    ":not([href$=\".ogg\"])",
    ":not([href$=\".woff\"])",
    ":not([href$=\".woff2\"])",
    ":not([href$=\".ttf\"])",
    ":not([href$=\".otf\"])",
    ":not([href$=\".swf\"])",
    ":not([href$=\".xap\"])",
    ":not([href$=\".ico\"])",
    ":not([href$=\".eot\"])",
    ":not([href$=\".bmp\"])",
    ":not([href$=\".psd\"])",
    ":not([href$=\".tiff\"])",
    ":not([href$=\".tif\"])",
    ":not([href$=\".heic\"])",
    ":not([href$=\".heif\"])",
    ":not([href$=\".mkv\"])",
    ":not([href$=\".webm\"])",
    ":not([href$=\".m4v\"])",
    ":not([href$=\".aac\"])",
    ":not([href$=\".flac\"])",
    ":not([href$=\".m4a\"])",
    ":not([href$=\".aiff\"])",
    ":not([href$=\".pdf\"])",
    ":not([href$=\".eps\"])",
    ":not([href$=\".yaml\"])",
    ":not([href$=\".yml\"])",
    ":not([href$=\".xml\"])",
    ":not([href$=\".css\"])",
    ":not([href$=\".js\"])",
    ":not([href$=\".txt\"])",
    ":not([href$=\".tar\"])",
    ":not([href$=\".doc\"])",
    ":not([href$=\".docx\"])",
    ":not([href$=\".zip\"])",
    ":not([href$=\".deb\"])",
    ":not([href$=\".pkg\"])",
    ":not([href$=\".tar.gz\"])",
    ":not([href$=\".rpm\"])",
    ":not([href$=\".z\"])",
    ":not([href$=\".7z\"])",
    ":not([href$=\".arj\"])",
    ":not([href$=\".rar\"])",
    ":not([href$=\".bin\"])",
    ":not([href$=\".msi\"])",
    ":not([href$=\".sh\"])",
    ":not([href$=\".bat\"])",
    ":not([href$=\".dmg\"])",
    ":not([href$=\".iso\"])",
    ":not([href$=\".toast\"])",
    ":not([href$=\".vcd\"])",
    ":not([href$=\".csv\"])",
    ":not([href$=\".log\"])",
    ":not([href$=\".sql\"])",
    ":not([href$=\".db\"])",
    ":not([href$=\".exe\"])",
    ":not([href$=\".rss\"])",
    ":not([href$=\".key\"])",
    ":not([href$=\".odp\"])",
    ":not([href$=\".pps\"])",
    ":not([href$=\".ptt\"])",
    ":not([href$=\".pptx\"])",
    ":not([href$=\".dump\"])",
);

type ScraperResult<T> = Result<T, Box<dyn Error>>;

pub struct ScrapedPage {
    pub title: Option<String>,
    pub favicon_url: Option<String>,
    pub content: Option<String>,
    pub html: Option<String>,
    pub html_length: usize,
    pub links: HashSet<String>,
    pub has_h1: bool,

    pub meta_description: Option<String>,
    pub meta_keywords: Option<String>,
    pub meta_theme_color: Option<String>,
    pub meta_og_image: Option<String>,
}

pub fn scrape_page(domain: String, url: String, page: String) -> ScraperResult<ScrapedPage> {
    let document = Html::parse_document(&page);
    let html = document.root_element().html();
    let selector = Selector::parse(LINK_SELECTOR)?;

    let mut links = HashSet::new();
    for element in document.select(&selector) {
        if let Some(link) = element.value().attr("href") {
            if let Ok(normalized_url) = normalize_href(&url, link) {
                if links.contains(&normalized_url) {
                    continue;
                }

                links.insert(normalized_url);
            }
        }
    }

    let title_selector = Selector::parse("title");
    let title = if let Ok(title_selector) = title_selector {
        document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
    } else {
        None
    };

    let h1_selector = Selector::parse("h1");
    let has_h1 = if let Ok(h1_selector) = h1_selector {
        document.select(&h1_selector).next().is_some()
    } else {
        false
    };

    let favicon_url = extract_favicon_url(domain, &document)?;
    let content = extract_text_content(&document)?;
    let content = if let Some(content) = content {
        if let Some(words) = extract_words(&content.to_lowercase()) {
            let text = words.join(" ");
            Some(safe_slice(&text, 1 << 7).to_string())
        } else {
            None
        }
    } else {
        None
    };

    let scraped = ScrapedPage {
        title,
        favicon_url,
        content,
        html: None,
        html_length: html.len(),
        links,
        has_h1,
        meta_description: extract_meta_content(&document, "description"),
        meta_keywords: extract_meta_content(&document, "keywords"),
        meta_theme_color: extract_meta_content(&document, "theme-color"),
        meta_og_image: extract_meta_content(&document, "og:image"),
    };

    Ok(scraped)
}

fn extract_favicon_url(domain: String, document: &Html) -> ScraperResult<Option<String>> {
    let selector = Selector::parse(r#"link[rel="icon"], link[rel="shortcut icon"]"#)?;

    if let Some(element) = document.select(&selector).next() {
        if let Some(href) = element.value().attr("href") {
            let favicon_url = if href.starts_with("http") {
                href.to_string()
            } else {
                normalize_href(&format!("https://{domain}"), &href.trim_start_matches('/'))?
            };

            return Ok(Some(favicon_url));
        }

        Ok(None)
    } else {
        Ok(None)
    }
}

fn extract_meta_content(document: &Html, name: &str) -> Option<String> {
    let meta_selector = Selector::parse("meta");

    if let Ok(meta_selector) = meta_selector {
        for element in document.select(&meta_selector) {
            if let Some(attr_name) = element.value().attr("name") {
                if attr_name == name {
                    return element
                        .value()
                        .attr("content")
                        .map(|x| x.trim())
                        .map(String::from);
                }
            }

            if let Some(attr_property) = element.value().attr("property") {
                if attr_property == name {
                    return element
                        .value()
                        .attr("content")
                        .map(|x| x.trim())
                        .map(String::from);
                }
            }
        }
    }

    None
}

fn extract_text_content(document: &Html) -> ScraperResult<Option<String>> {
    let body_selector = Selector::parse("body");

    if let Ok(body_selector) = body_selector {
        let mut text_content = String::new();

        if let Some(body) = document.select(&body_selector).next() {
            let ignore_selector = Selector::parse("script, style, noscript")?;

            for node in body.select(&Selector::parse("*")?) {
                if !ignore_selector.matches(&node) {
                    let text = node.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    if !text.is_empty() {
                        text_content.push_str(&text);
                        text_content.push(' ');
                    }
                }
            }
        }

        return Ok(Some(text_content.trim().to_string()));
    }

    Ok(None)
}
