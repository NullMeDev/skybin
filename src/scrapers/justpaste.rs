use super::traits::{Scraper, ScraperError, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// JustPaste.it scraper
/// JustPaste.it is a popular anonymous paste service with rich formatting
pub struct JustPasteScraper {
    base_url: String,
}

impl JustPasteScraper {
    pub fn new() -> Self {
        JustPasteScraper {
            base_url: "https://justpaste.it".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        JustPasteScraper { base_url: url }
    }
}

impl Default for JustPasteScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for JustPasteScraper {
    fn name(&self) -> &str {
        "justpaste"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // JustPaste.it doesn't have a public recent/archive page
        // Pastes are accessed via short IDs like https://justpaste.it/abc123
        //
        // The site is heavily used for:
        // - Text sharing/notes
        // - Anonymous document sharing
        // - Sometimes data leaks
        //
        // Discovery via external URL submission is recommended

        let response = client
            .get(&self.base_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ScraperError::SourceUnavailable(format!(
                "JustPaste.it returned {}",
                response.status()
            )));
        }

        // No public listing available
        tracing::debug!(
            "JustPaste.it: No public recent API. Use /api/submit-url to monitor specific pastes."
        );

        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_justpaste_scraper_creation() {
        let scraper = JustPasteScraper::new();
        assert_eq!(scraper.name(), "justpaste");
    }

    #[test]
    fn test_justpaste_scraper_default() {
        let scraper = JustPasteScraper::default();
        assert_eq!(scraper.name(), "justpaste");
    }

    #[test]
    fn test_justpaste_custom_url() {
        let custom_url = "https://jp.custom.com".to_string();
        let scraper = JustPasteScraper::with_url(custom_url.clone());
        assert_eq!(scraper.base_url, custom_url);
    }
}
