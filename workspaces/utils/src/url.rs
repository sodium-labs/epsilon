use url::{ParseError, Url};

pub fn normalize_url(url: &str) -> Option<(Url, String)> {
    if let Ok(mut normalized_url) = Url::parse(url) {
        normalized_url.set_query(None);
        normalized_url.set_fragment(None);
        if let Some(domain) = normalized_url.clone().domain() {
            Some((normalized_url, domain.to_string()))
        } else {
            None
        }
    } else {
        None
    }
}

/// Normalize a website href link
///
/// `base` is the page url, and `link` the string inside the `href` attribute of an `a` element.
///
/// The link can be absolute or relative. The function will return the absolute url.
pub fn normalize_href(base: &str, link: &str) -> Result<String, ParseError> {
    if link.starts_with("http") {
        let mut normalized_url = Url::parse(link)?;

        normalized_url.set_query(None);
        normalized_url.set_fragment(None);

        return Ok(normalized_url.to_string());
    }

    let base_url = Url::parse(base)?;
    let mut normalized_url = base_url.join(link)?;

    normalized_url.set_query(None);
    normalized_url.set_fragment(None);

    Ok(normalized_url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("google").is_none(), true);
        assert_eq!(normalize_url("google.com").is_none(), true);
        assert_eq!(normalize_url("/google.com").is_none(), true);
        assert_eq!(normalize_url("//google.com").is_none(), true);
        assert_eq!(
            normalize_url("https://google.com").unwrap().0.to_string(),
            "https://google.com/"
        );
        assert_eq!(
            normalize_url("https://google.com/about#cc?a=0")
                .unwrap()
                .0
                .to_string(),
            "https://google.com/about"
        );
        assert_eq!(
            normalize_url("https://google.com/about?a")
                .unwrap()
                .0
                .to_string(),
            "https://google.com/about"
        );
    }

    #[test]
    fn test_normalize_href() {
        assert_eq!(
            normalize_href("https://google.com", "/about").unwrap(),
            "https://google.com/about"
        );
        assert_eq!(
            normalize_href("https://google.com", "https://wikipedia.org").unwrap(),
            "https://wikipedia.org/"
        );
        assert_eq!(
            normalize_href("https://sub.google.com", "hello").unwrap(),
            "https://sub.google.com/hello"
        );
        assert_eq!(
            normalize_href("https://google.com", "#a").unwrap(),
            "https://google.com/"
        );
        assert_eq!(
            normalize_href("https://google.com", "sftp://example.com").unwrap(),
            "sftp://example.com"
        );
    }
}
