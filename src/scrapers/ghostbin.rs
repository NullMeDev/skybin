use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// Ghostbin scraper
/// Note: Original ghostbin.com is defunct. This scraper is a placeholder
/// that could be adapted to work with ghostbin alternatives or mirrors.
pub struct GhostbinScraper {
    #[allow(dead_code)]
    base_url: String,
}

impl GhostbinScraper {
    pub fn new() -> Self {
        GhostbinScraper {
            // ghostbin.com is defunct - using placeholder URL
            base_url: "https://ghostbin.com".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        GhostbinScraper { base_url: url }
    }
}

impl Default for GhostbinScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for GhostbinScraper {
    fn name(&self) -> &str {
        "ghostbin"
    }

    async fn fetch_recent(&self, _client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Ghostbin.com is defunct (shut down in 2021)
        // This is a placeholder implementation
        //
        // Potential alternatives that could be added:
        // - ghostbin.co (if exists)
        // - Other similar anonymous paste services
        //
        // For now, return empty to avoid errors
        tracing::info!("Ghostbin scraper: Original service is defunct, returning empty");
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ghostbin_scraper_creation() {
        let scraper = GhostbinScraper::new();
        assert_eq!(scraper.name(), "ghostbin");
    }

    #[test]
    fn test_ghostbin_scraper_default() {
        let scraper = GhostbinScraper::default();
        assert_eq!(scraper.name(), "ghostbin");
    }

    #[test]
    fn test_ghostbin_custom_url() {
        let custom_url = "https://ghostbin.co".to_string();
        let scraper = GhostbinScraper::with_url(custom_url.clone());
        assert_eq!(scraper.base_url, custom_url);
    }
}
