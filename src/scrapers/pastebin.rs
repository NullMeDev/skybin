use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct PastebinItem {
    key: String,
    title: Option<String>,
    user: Option<String>,
    size: String,
    expire_date: Option<i64>,
    private: String,
    syntax: Option<String>,
    created: i64,
}

/// Pastebin scraper using the public API
pub struct PastebinScraper {
    api_url: String,
}

impl PastebinScraper {
    pub fn new() -> Self {
        PastebinScraper {
            api_url: "https://scrape.pastebin.com/api_scraping.php".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        PastebinScraper { api_url: url }
    }
}

impl Default for PastebinScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for PastebinScraper {
    fn name(&self) -> &str {
        "pastebin"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let response = client
            .get(&self.api_url)
            .query(&[("limit", "10")])
            .send()
            .await?;

        let items: Vec<PastebinItem> = response.json().await?;

        let pastes = items
            .into_iter()
            .map(|item| {
                DiscoveredPaste::new("pastebin", &item.key, format!("Pastebin-{}", item.key))
                    .with_title(item.title.unwrap_or_default())
                    .with_author(item.user.unwrap_or_default())
                    .with_url(format!("https://pastebin.com/{}", item.key))
                    .with_syntax(item.syntax.unwrap_or_else(|| "text".to_string()))
                    .with_created_at(item.created)
            })
            .collect();

        Ok(pastes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pastebin_scraper_creation() {
        let scraper = PastebinScraper::new();
        assert_eq!(scraper.name(), "pastebin");
    }

    #[test]
    fn test_pastebin_scraper_default() {
        let scraper = PastebinScraper::default();
        assert_eq!(scraper.name(), "pastebin");
    }

    #[test]
    fn test_pastebin_custom_url() {
        let custom_url = "https://custom.api.pastebin.com".to_string();
        let scraper = PastebinScraper::with_url(custom_url.clone());
        assert_eq!(scraper.api_url, custom_url);
    }
}
