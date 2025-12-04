use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// DPaste scraper
/// Note: dpaste.com doesn't have a public recent pastes API
pub struct DPasteScraper {
    #[allow(dead_code)]
    base_url: String,
}

impl DPasteScraper {
    pub fn new() -> Self {
        DPasteScraper {
            base_url: "https://dpaste.com".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        DPasteScraper { base_url: url }
    }
}

impl Default for DPasteScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for DPasteScraper {
    fn name(&self) -> &str {
        "dpaste"
    }

    async fn fetch_recent(&self, _client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // dpaste.com doesn't have a public recent/archive page
        // Pastes are private by default and there's no public listing API
        // Users should submit specific dpaste URLs via /api/submit-url
        tracing::debug!("dpaste.com: No public recent API available");
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpaste_scraper_creation() {
        let scraper = DPasteScraper::new();
        assert_eq!(scraper.name(), "dpaste");
    }

    #[test]
    fn test_dpaste_scraper_default() {
        let scraper = DPasteScraper::default();
        assert_eq!(scraper.name(), "dpaste");
    }

    #[test]
    fn test_dpaste_custom_url() {
        let custom_url = "https://custom.dpaste.com".to_string();
        let scraper = DPasteScraper::with_url(custom_url.clone());
        assert_eq!(scraper.base_url, custom_url);
    }
}
