use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// Rentry scraper - scrapes from rentry.co recent page
pub struct RentryScraper {
    #[allow(dead_code)]
    base_url: String,
}

impl RentryScraper {
    pub fn new() -> Self {
        RentryScraper {
            base_url: "https://rentry.co".to_string(),
        }
    }
}

impl Default for RentryScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for RentryScraper {
    fn name(&self) -> &str {
        "rentry"
    }

    async fn fetch_recent(&self, _client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Rentry.co doesn't have a public recent page, so we'll return empty for now
        // In production, you'd need to either:
        // 1. Have a list of known rentry URLs to check
        // 2. Use an API if they provide one
        // 3. Monitor social media/forums for rentry links
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rentry_scraper_creation() {
        let scraper = RentryScraper::new();
        assert_eq!(scraper.name(), "rentry");
    }
}
