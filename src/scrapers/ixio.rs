use super::traits::{Scraper, ScraperError, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// ix.io scraper
/// ix.io is a simple command-line pastebin service
/// Pastes can be accessed via http://ix.io/PASTE_ID
pub struct IxioScraper {
    base_url: String,
}

impl IxioScraper {
    pub fn new() -> Self {
        IxioScraper {
            base_url: "http://ix.io".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        IxioScraper { base_url: url }
    }
}

impl Default for IxioScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for IxioScraper {
    fn name(&self) -> &str {
        "ixio"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // ix.io doesn't have a public recent page
        // Pastes are accessed via sequential numeric IDs (base62 encoded)
        //
        // Discovery strategies:
        // 1. Monitor known users' paste lists (http://ix.io/user/USERNAME)
        // 2. Try sequential IDs (not recommended - too many requests)
        // 3. Use external URL submission API instead
        //
        // For now, check if site is available
        let response = client
            .get(&self.base_url)
            .header("User-Agent", "SkyBin/2.1.0 (security research)")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ScraperError::SourceUnavailable(format!(
                "ix.io returned {}",
                response.status()
            )));
        }

        // ix.io has no public listing - users should submit specific URLs
        // via the external URL API: POST /api/submit-url
        tracing::debug!(
            "ix.io: No public recent API. Use /api/submit-url to monitor specific pastes."
        );

        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ixio_scraper_creation() {
        let scraper = IxioScraper::new();
        assert_eq!(scraper.name(), "ixio");
    }

    #[test]
    fn test_ixio_scraper_default() {
        let scraper = IxioScraper::default();
        assert_eq!(scraper.name(), "ixio");
    }

    #[test]
    fn test_ixio_custom_url() {
        let custom_url = "https://ix.io".to_string();
        let scraper = IxioScraper::with_url(custom_url.clone());
        assert_eq!(scraper.base_url, custom_url);
    }
}
