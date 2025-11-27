use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// Hastebin scraper for toptal.com/developers/hastebin
/// Note: Hastebin doesn't have a public API for listing recent pastes
/// This scraper attempts to access commonly used paste IDs or implements
/// a lightweight discovery mechanism
pub struct HastebinScraper {
    #[allow(dead_code)]
    base_url: String,
}

impl HastebinScraper {
    pub fn new() -> Self {
        HastebinScraper {
            base_url: "https://www.toptal.com/developers/hastebin".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        HastebinScraper { base_url: url }
    }
}

impl Default for HastebinScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for HastebinScraper {
    fn name(&self) -> &str {
        "hastebin"
    }

    async fn fetch_recent(&self, _client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Hastebin doesn't have a public recent pastes API
        // This is a placeholder implementation that could be extended with:
        // 1. Monitoring specific known paste IDs
        // 2. Using a database of previously discovered IDs
        // 3. Implementing ID generation/guessing (ethically questionable)

        // For now, return empty vector to avoid errors
        // Future: Implement discovery mechanism or monitor from other sources
        tracing::warn!("Hastebin scraper: No public API available for recent pastes");
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hastebin_scraper_creation() {
        let scraper = HastebinScraper::new();
        assert_eq!(scraper.name(), "hastebin");
    }

    #[test]
    fn test_hastebin_scraper_default() {
        let scraper = HastebinScraper::default();
        assert_eq!(scraper.name(), "hastebin");
    }

    #[test]
    fn test_hastebin_custom_url() {
        let custom_url = "https://hastebin.com".to_string();
        let scraper = HastebinScraper::with_url(custom_url.clone());
        assert_eq!(scraper.base_url, custom_url);
    }
}
