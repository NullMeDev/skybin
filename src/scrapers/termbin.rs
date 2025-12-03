use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use super::traits::{Scraper, ScraperResult};

pub struct TermbinScraper;

impl TermbinScraper {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TermbinScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for TermbinScraper {
    fn name(&self) -> &str {
        "termbin"
    }

    async fn fetch_recent(&self, _client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Termbin doesn't have a public archive, but we can monitor known paste IDs
        // This scraper is placeholder for URL submissions
        Ok(Vec::new())
    }
}
