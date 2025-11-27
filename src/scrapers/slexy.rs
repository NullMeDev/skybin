use super::traits::{Scraper, ScraperError, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// Slexy.org scraper
/// Slexy has a recent pastes page at /recent
pub struct SlexyScraper {
    base_url: String,
}

impl SlexyScraper {
    pub fn new() -> Self {
        SlexyScraper {
            base_url: "https://slexy.org".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        SlexyScraper { base_url: url }
    }
}

impl Default for SlexyScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for SlexyScraper {
    fn name(&self) -> &str {
        "slexy"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let recent_url = format!("{}/recent", self.base_url);

        let response = client
            .get(&recent_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ScraperError::SourceUnavailable(format!(
                "Slexy returned {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let mut pastes = Vec::new();

        // Parse paste links from recent page
        // Format: <a href="/view/PASTE_ID">
        let re = regex::Regex::new(r#"<a href="/view/([a-zA-Z0-9]+)"[^>]*>"#).unwrap();

        for cap in re.captures_iter(&html).take(10) {
            let paste_id = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            if paste_id.is_empty() {
                continue;
            }

            // Fetch raw paste content
            let raw_url = format!("{}/raw/{}", self.base_url, paste_id);
            match client
                .get(&raw_url)
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                )
                .send()
                .await
            {
                Ok(content_response) => {
                    if content_response.status().is_success() {
                        if let Ok(content) = content_response.text().await {
                            if !content.is_empty() && content.len() < 100000 {
                                let paste = DiscoveredPaste::new("slexy", paste_id, content)
                                    .with_url(format!("{}/view/{}", self.base_url, paste_id))
                                    .with_title(format!("Slexy: {}", paste_id));
                                pastes.push(paste);
                                tracing::debug!("Fetched slexy paste: {}", paste_id);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch slexy paste {}: {}", paste_id, e);
                    continue;
                }
            }

            // Rate limit
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Ok(pastes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slexy_scraper_creation() {
        let scraper = SlexyScraper::new();
        assert_eq!(scraper.name(), "slexy");
    }

    #[test]
    fn test_slexy_scraper_default() {
        let scraper = SlexyScraper::default();
        assert_eq!(scraper.name(), "slexy");
    }

    #[test]
    fn test_slexy_custom_url() {
        let custom_url = "https://custom.slexy.org".to_string();
        let scraper = SlexyScraper::with_url(custom_url.clone());
        assert_eq!(scraper.base_url, custom_url);
    }
}
