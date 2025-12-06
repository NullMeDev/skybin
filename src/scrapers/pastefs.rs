use crate::models::DiscoveredPaste;
use crate::scrapers::traits::{Scraper, ScraperResult};
use async_trait::async_trait;

pub struct PasteFsScraper;

impl PasteFsScraper {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Scraper for PasteFsScraper {
    fn name(&self) -> &str {
        "pastefs"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // PasteFS (pastefs.com) - simple paste site with recent feed
        let url = "https://pastefs.com/api/recent";
        
        let response = client
            .get(url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::scrapers::ScraperError::Other(format!("HTTP {}", response.status())));
        }

        #[derive(serde::Deserialize)]
        struct PasteItem {
            id: String,
            #[serde(default)]
            title: Option<String>,
            #[serde(default)]
            language: Option<String>,
        }

        let items: Vec<PasteItem> = response.json().await?;
        let mut pastes = Vec::new();

        for item in items.into_iter().take(20) {
            // Fetch full content
            let content_url = format!("https://pastefs.com/raw/{}", item.id);
            match client
                .get(&content_url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(content) = resp.text().await {
                        if !content.trim().is_empty() && content.len() < 500_000 {
                            let paste = DiscoveredPaste::new("pastefs", &item.id, content)
                                .with_title(item.title.unwrap_or_else(|| "Untitled".to_string()))
                                .with_url(format!("https://pastefs.com/paste/{}", item.id))
                                .with_syntax(item.language.unwrap_or_else(|| "plaintext".to_string()));
                            pastes.push(paste);
                        }
                    }
                }
                _ => continue,
            }

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        Ok(pastes)
    }
}

impl Default for PasteFsScraper {
    fn default() -> Self {
        Self::new()
    }
}
