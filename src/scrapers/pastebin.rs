use super::credential_filter::contains_credentials;
use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// Pastebin scraper using archive page and scraping API
pub struct PastebinScraper {
    archive_url: String,
    #[allow(dead_code)]
    api_key: Option<String>,
}

impl PastebinScraper {
    pub fn new() -> Self {
        PastebinScraper {
            archive_url: "https://pastebin.com/archive".to_string(),
            api_key: None,
        }
    }

    pub fn with_api_key(api_key: String) -> Self {
        PastebinScraper {
            archive_url: "https://pastebin.com/archive".to_string(),
            api_key: if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            },
        }
    }

    pub fn with_url(url: String) -> Self {
        PastebinScraper {
            archive_url: url,
            api_key: None,
        }
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
        // Fetch archive page HTML
        let response = client
            .get(&self.archive_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::scrapers::ScraperError::SourceUnavailable(format!(
                "Pastebin archive returned {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let mut pastes = Vec::new();

        // Extract paste IDs from archive page using regex
        // Format: <a href="/PASTE_ID">Title</a>
        let re = regex::Regex::new(r#"<a href="/([a-zA-Z0-9]{8})"[^>]*>([^<]+)</a>"#).unwrap();

        // Get up to 30 pastes per scrape - NO LIMITS
        for cap in re.captures_iter(&html).take(30) {
            let paste_id = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let title = cap.get(2).map(|m| m.as_str()).unwrap_or("Untitled");

            if paste_id.is_empty() {
                continue;
            }

            // Fetch actual paste content
            let raw_url = format!("https://pastebin.com/raw/{}", paste_id);
            match client
                .get(&raw_url)
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                .header("Accept-Language", "en-US,en;q=0.5")
                .send()
                .await
            {
                Ok(content_response) => {
                    if content_response.status().is_success() {
                        if let Ok(content) = content_response.text().await {
                            // Only keep pastes with credentials
                            if !content.is_empty() && contains_credentials(&content) {
                                let paste = DiscoveredPaste::new("pastebin", paste_id, content)
                                    .with_title(title.to_string())
                                    .with_url(format!("https://pastebin.com/{}", paste_id))
                                    .with_syntax("plaintext".to_string());
                                pastes.push(paste);
                            }
                        }
                    }
                }
                Err(_) => continue,
            }

            // Minimal delay to avoid rate limits
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }

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
        let custom_url = "https://custom.pastebin.com/archive".to_string();
        let scraper = PastebinScraper::with_url(custom_url.clone());
        assert_eq!(scraper.archive_url, custom_url);
    }
}
