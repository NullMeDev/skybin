use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct PasteEeItem {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    views: u32,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    created: Option<i64>,
    #[serde(default)]
    modified: Option<i64>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PasteEeResponse {
    #[serde(default)]
    pastes: Option<Vec<PasteEeItem>>,
    #[serde(default)]
    items: Option<Vec<PasteEeItem>>,
}

/// Paste.ee scraper using the public API
/// Fetches recently added public pastes
pub struct PasteEeScraper {
    api_url: String,
}

impl PasteEeScraper {
    pub fn new() -> Self {
        PasteEeScraper {
            api_url: "https://paste.ee/api".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        PasteEeScraper { api_url: url }
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

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let url = format!("{}/recent", self.api_url);

        let response = client
            .get(&url)
            .query(&[("limit", "25")])
            .header(
                "User-Agent",
                "SkyBin-PasteEe-Scraper/1.0 (anonymous content aggregator)",
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::scrapers::ScraperError::SourceUnavailable(format!(
                "Paste.ee API returned {}",
                response.status()
            )));
        }

        let data: PasteEeResponse = response.json().await?;
        let items = data.pastes.or(data.items).unwrap_or_default();

        let mut pastes = Vec::new();

        for item in items {
            // Skip if no ID
            if item.id.is_empty() {
                continue;
            }

            // Create a title from provided title or description
            let title = item
                .title
                .or(item.description)
                .unwrap_or_else(|| format!("Paste.ee-{}", item.id));

            // Note: We intentionally don't set author here - it will be None before storage
            // This is per anonymization requirements
            let paste = DiscoveredPaste::new(
                "paste_ee",
                &item.id,
                // Content would normally come from fetching the full paste,
                // but for now we use title as placeholder (to be updated in actual implementation)
                // In production, this would fetch the full paste content
                format!("Title: {}", title),
            )
            .with_title(title)
            .with_url(format!("https://paste.ee/p/{}", item.id))
            .with_syntax(
                item.language
                    .unwrap_or_else(|| "text".to_string()),
            )
            .with_created_at(
                item.created
                    .or(item.modified)
                    .unwrap_or(0),
            );

            pastes.push(paste);
        }

        Ok(pastes)
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
        let custom_url = "https://custom.paste.ee/api".to_string();
        let scraper = PasteEeScraper::with_url(custom_url.clone());
        assert_eq!(scraper.api_url, custom_url);
    }
}
