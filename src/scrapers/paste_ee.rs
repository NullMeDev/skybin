use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// Paste.ee scraper - scrapes from paste.ee web interface
/// Note: Paste.ee doesn't have a public recent/archive page
pub struct PasteEeScraper {
    base_url: String,
}

impl PasteEeScraper {
    pub fn new() -> Self {
        PasteEeScraper {
            base_url: "https://paste.ee".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        PasteEeScraper { base_url: url }
    }
}

impl Default for PasteEeScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for PasteEeScraper {
    fn name(&self) -> &str {
        "paste_ee"
    }

    async fn fetch_recent(&self, _client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Paste.ee doesn't have a public recent/archive page
        // Pastes are private by default and there's no listing
        // Users should submit specific paste.ee URLs via /api/submit-url
        tracing::debug!("Paste.ee: No public recent API available");
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paste_ee_scraper_creation() {
        let scraper = PasteEeScraper::new();
        assert_eq!(scraper.name(), "paste_ee");
    }

    #[test]
    fn test_paste_ee_scraper_default() {
        let scraper = PasteEeScraper::default();
        assert_eq!(scraper.name(), "paste_ee");
    }

    #[test]
    fn test_paste_ee_custom_url() {
        let custom_url = "https://custom.paste.ee".to_string();
        let scraper = PasteEeScraper::with_url(custom_url.clone());
        assert_eq!(scraper.base_url, custom_url);
    }
}
