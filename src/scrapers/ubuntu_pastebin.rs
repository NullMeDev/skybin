use super::traits::{Scraper, ScraperError, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// Ubuntu Pastebin scraper (paste.ubuntu.com)
/// Popular paste site for the Ubuntu community
pub struct UbuntuPastebinScraper {
    base_url: String,
}

impl UbuntuPastebinScraper {
    pub fn new() -> Self {
        UbuntuPastebinScraper {
            base_url: "https://paste.ubuntu.com".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        UbuntuPastebinScraper { base_url: url }
    }
}

impl Default for UbuntuPastebinScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for UbuntuPastebinScraper {
    fn name(&self) -> &str {
        "ubuntu_pastebin"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Ubuntu Pastebin doesn't have a public recent page
        // We can only access pastes if we know their IDs
        // This is a placeholder that could monitor specific IDs or use discovery

        let response = client
            .get(&self.base_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Ubuntu; Linux x86_64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ScraperError::SourceUnavailable(format!(
                "Ubuntu Pastebin returned {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let mut pastes = Vec::new();

        // Try to find paste links on the main page
        // Format: <a href="/p/PASTE_ID/">
        let re = regex::Regex::new(r#"<a href="/p/([a-zA-Z0-9]+)/"#).unwrap();

        for cap in re.captures_iter(&html).take(10) {
            let paste_id = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            if paste_id.is_empty() {
                continue;
            }

            // Fetch raw paste content
            let raw_url = format!("{}/p/{}/plain/", self.base_url, paste_id);
            match client
                .get(&raw_url)
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64) AppleWebKit/537.36",
                )
                .send()
                .await
            {
                Ok(content_response) => {
                    if content_response.status().is_success() {
                        if let Ok(content) = content_response.text().await {
                            if !content.is_empty() && content.len() < 100000 {
                                let paste =
                                    DiscoveredPaste::new("ubuntu_pastebin", paste_id, content)
                                        .with_url(format!("{}/p/{}/", self.base_url, paste_id))
                                        .with_title(format!("Ubuntu Paste: {}", paste_id));
                                pastes.push(paste);
                                tracing::debug!("Fetched ubuntu paste: {}", paste_id);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch ubuntu paste {}: {}", paste_id, e);
                    continue;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        if pastes.is_empty() {
            tracing::info!("Ubuntu Pastebin: No public recent pastes API available");
        }

        Ok(pastes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ubuntu_pastebin_scraper_creation() {
        let scraper = UbuntuPastebinScraper::new();
        assert_eq!(scraper.name(), "ubuntu_pastebin");
    }

    #[test]
    fn test_ubuntu_pastebin_scraper_default() {
        let scraper = UbuntuPastebinScraper::default();
        assert_eq!(scraper.name(), "ubuntu_pastebin");
    }

    #[test]
    fn test_ubuntu_pastebin_custom_url() {
        let custom_url = "https://paste.custom.com".to_string();
        let scraper = UbuntuPastebinScraper::with_url(custom_url.clone());
        assert_eq!(scraper.base_url, custom_url);
    }
}
