use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

pub struct SprungeScraper;

impl SprungeScraper {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SprungeScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for SprungeScraper {
    fn name(&self) -> &str {
        "sprunge"
    }

    async fn fetch_recent(&self, _client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Sprunge.us is CLI-focused, no public recent feed
        Ok(Vec::new())
    }
}
