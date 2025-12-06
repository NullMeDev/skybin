use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

pub struct PasteRsScraper;

impl PasteRsScraper {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PasteRsScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for PasteRsScraper {
    fn name(&self) -> &str {
        "paste_rs"
    }

    async fn fetch_recent(&self, _client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // paste.rs is CLI-focused without public archive
        Ok(Vec::new())
    }
}
