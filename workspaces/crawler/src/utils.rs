use crate::scraper::ScrapedPage;
use regex::Regex;
use reqwest::header::HeaderMap;
use url::Url;

/// Validate that a link is a valid URL and starts with http/https
pub fn is_crawlable_url(link: &str) -> bool {
    if let Ok(url) = Url::parse(link) {
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return false;
        }
        return true;
    }
    false
}

pub fn extract_words(text: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    if let Ok(regex) = Regex::new(r"\b[a-zä-ÿ]{2,}\b") {
        for cap in regex.captures_iter(text) {
            let word = cap[0].to_lowercase();
            words.push(word);
        }

        Some(words)
    } else {
        None
    }
}

pub fn get_content_type<'a>(headers: &HeaderMap, url: &str) -> Option<&'a str> {
    if let Some(value) = headers.get("content-type") {
        let clean_type = if let Ok(value) = value.to_str() {
            value.split(';').next().unwrap_or("").trim()
        } else {
            ""
        };

        if clean_type == "text/html" {
            Some("text/html")
        } else {
            None
        }
    } else {
        if url.ends_with(".html") || url.ends_with(".htm") {
            Some("text/html")
        } else {
            None
        }
    }
}

pub fn calculate_seo_score(scraped: &ScrapedPage) -> i32 {
    let mut seo_score = 0;
    if scraped.title.is_some() {
        seo_score += 25;
    }
    if let Some(desc) = &scraped.meta_description {
        seo_score += 20;

        if desc.len() >= 50 && desc.len() <= 160 {
            seo_score += 5;
        }
    }
    if scraped.meta_keywords.is_some() {
        seo_score += 20;
    }
    if scraped.meta_og_image.is_some() {
        seo_score += 10
    }
    if scraped.has_h1 {
        seo_score += 10;
    }
    if scraped.links.len() >= 5 {
        seo_score += 10;
    }
    seo_score.clamp(0, 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_crawlable_url() {
        assert_eq!(is_crawlable_url("https://google.com"), true);
        assert_eq!(is_crawlable_url("http://google.com"), true);
        assert_eq!(is_crawlable_url("https://google.com/hello"), true);
        assert_eq!(is_crawlable_url("sftp://google.com"), false);
        assert_eq!(is_crawlable_url("ws://google.com"), false);
        assert_eq!(is_crawlable_url("wss://google.com"), false);
    }

    #[test]
    fn test_extract_words() {
        assert_eq!(extract_words("a b c 1 2 3").unwrap(), Vec::<String>::new());
        assert_eq!(extract_words("aa b c 1 2 3456").unwrap(), vec!["aa"]);
        assert_eq!(
            extract_words("Hello my friend!").unwrap(),
            vec!["my", "friend"]
        );
        assert_eq!(extract_words("t1a2b3").unwrap(), Vec::<String>::new());
        assert_eq!(
            extract_words("This1 is2 very strange3").unwrap(),
            vec!["very"]
        );
        assert_eq!(
            extract_words("Wii sports resort is the BEST game ever!").unwrap(),
            vec!["sports", "resort", "is", "the", "game", "ever"]
        );
        assert_eq!(
            extract_words("This, should. work, i. think!").unwrap(),
            vec!["should", "work", "think"]
        );
    }
}
