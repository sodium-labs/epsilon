use reqwest::Client;
use robotstxt::DefaultMatcher;
use std::time::Instant;

/// Cooldown before crawling the robots again
pub const ROBOTS_FETCH_COOLDOWN: u128 = 86_400_000;

pub struct Website {
    pub domain: String,
    pub robots: Option<String>,
    pub last_robots_fetch: Option<Instant>,
    pub last_crawl: Option<Instant>,
}

impl Website {
    pub fn new(domain: String) -> Self {
        Self {
            domain,
            robots: None,
            last_robots_fetch: None,
            last_crawl: None,
        }
    }

    pub fn should_fetch_robots(&self) -> bool {
        if let Some(last_fetch) = self.last_robots_fetch {
            if last_fetch.elapsed().as_millis() >= ROBOTS_FETCH_COOLDOWN {
                return true;
            }
        } else {
            return true;
        }
        false
    }

    pub async fn fetch_robots(domain: String, client: &Client) -> Result<Option<String>, reqwest::Error> {
        let robots_url = format!("https://{}/robots.txt", domain);
        let response = client.get(robots_url).send().await?;
        let response_status = response.status();

        if !response_status.is_success() {
            return Ok(None);
        }

        let text = response.text().await?;

        // Cancel if it looks like a html file
        if text.starts_with("<") {
            return Ok(None);
        }

        Ok(Some(text))
    }

    pub fn set_robots(&mut self, text: Option<String>) {
        self.last_robots_fetch = Some(Instant::now());
        self.robots = text;
    }

    pub fn is_crawlable(&self, user_agent: &str, url: &str) -> bool {
        if let Some(robots) = &self.robots {
            let mut matcher = DefaultMatcher::default();
            matcher.one_agent_allowed_by_robots(robots, user_agent, url)
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_crawlable() {
        let mut website = Website::new("google.com".into());

        website.robots =
            Some("User-agent: *\nDisallow: /api\n\nSitemap: https://google.com/sitemap.xml".into());
        assert_eq!(website.is_crawlable("Epsilon", "/"), true);
        assert_eq!(website.is_crawlable("Epsilon", "/hello"), true);
        assert_eq!(website.is_crawlable("Epsilon", "/api"), false);
        assert_eq!(website.is_crawlable("Epsilon", "/api/v0"), false);

        website.robots = Some("User-agent: *\nDisallow: /api\n\nUser-agent: Epsilon\nDisallow: /home\n\nSitemap: https://google.com/sitemap.xml".into());
        assert_eq!(website.is_crawlable("Other", "/"), true);
        assert_eq!(website.is_crawlable("Other", "/home"), true);
        assert_eq!(website.is_crawlable("Other", "/api"), false);
        assert_eq!(website.is_crawlable("Epsilon", "/"), true);
        assert_eq!(website.is_crawlable("Epsilon", "/home"), false);
        assert_eq!(website.is_crawlable("Epsilon", "/api"), true);
    }
}
