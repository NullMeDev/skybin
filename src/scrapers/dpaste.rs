use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct DPasteItem {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    author_name: Option<String>,
    #[serde(default)]
    syntax: Option<String>,
    #[serde(default)]
    created: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DPasteResponse {
    #[serde(default)]
    results: Vec<DPasteItem>,
    #[serde(default)]
    pastes: Vec<DPasteItem>,
}

/// DPaste scraper
/// Fetches recently added public pastes from DPaste
pub struct DPasteScraper {
    api_url: String,
}

impl DPasteScraper {
    pub fn new() -> Self {
        DPasteScraper {
            api_url: "https://dpaste.com/api/v2".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        DPasteScraper { api_url: url }
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

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // DPaste doesn't have a standard public API for recent pastes,
        // so we'll implement a lightweight scraper that queries the recent feed
        let url = format!("{}/list", self.api_url);

        let response = client
            .get(&url)
            .query(&[("limit", "20")])
            .header(
                "User-Agent",
                "SkyBin-DPaste-Scraper/1.0 (anonymous content aggregator)",
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::scrapers::ScraperError::SourceUnavailable(format!(
                "DPaste API returned {}",
                response.status()
            )));
        }

        let data: DPasteResponse = match response.json().await {
            Ok(d) => d,
            Err(_) => {
                // Fallback if JSON parsing fails
                return Ok(Vec::new());
            }
        };

        let items = if !data.results.is_empty() {
            data.results
        } else {
            data.pastes
        };

        let mut pastes = Vec::new();

        for item in items {
            // Skip if no ID
            if item.id.is_empty() {
                continue;
            }

            // Skip if no content
            let content = match item.content {
                Some(c) if !c.is_empty() => c,
                _ => continue,
            };

            // Create title from provided title or use ID
            let title = item.title.unwrap_or_else(|| format!("DPaste-{}", item.id));

            // Note: We intentionally don't set author here - it will be None before storage
            // This is per anonymization requirements
            let paste = DiscoveredPaste::new("dpaste", &item.id, content)
                .with_title(title)
                .with_url(
                    item.url
                        .unwrap_or_else(|| format!("https://dpaste.com/{}", item.id)),
                )
                .with_syntax(item.syntax.unwrap_or_else(|| "text".to_string()))
                .with_created_at(
                    // Parse ISO 8601 timestamp if available
                    item.created
                        .as_ref()
                        .and_then(|dt| {
                            chrono::DateTime::parse_from_rfc3339(dt)
                                .ok()
                                .map(|d| d.timestamp())
                        })
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
        let custom_url = "https://custom.dpaste.com/api".to_string();
        let scraper = DPasteScraper::with_url(custom_url.clone());
        assert_eq!(scraper.api_url, custom_url);
    }
}
