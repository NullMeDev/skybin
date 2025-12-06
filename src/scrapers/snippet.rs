use crate::models::DiscoveredPaste;
use crate::scrapers::traits::{Scraper, ScraperResult};
use async_trait::async_trait;

pub struct SnippetScraper;

impl SnippetScraper {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Scraper for SnippetScraper {
    fn name(&self) -> &str {
        "snippet"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Snippet.host - minimalist paste service
        let url = "https://snippet.host/api/recent";
        
        let response = client
            .get(url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::scrapers::ScraperError::Other(format!("HTTP {}", response.status())));
        }

        #[derive(serde::Deserialize)]
        struct SnippetItem {
            id: String,
            #[serde(default)]
            title: Option<String>,
            #[serde(default)]
            lang: Option<String>,
        }

        let items: Vec<SnippetItem> = response.json().await?;
        let mut pastes = Vec::new();

        for item in items.into_iter().take(20) {
            // Fetch raw content
            let raw_url = format!("https://snippet.host/raw/{}", item.id);
            match client
                .get(&raw_url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(content) = resp.text().await {
                        if !content.trim().is_empty() && content.len() < 500_000 {
                            let paste = DiscoveredPaste::new("snippet", &item.id, content)
                                .with_title(item.title.unwrap_or_else(|| "Snippet".to_string()))
                                .with_url(format!("https://snippet.host/{}", item.id))
                                .with_syntax(item.lang.unwrap_or_else(|| "plaintext".to_string()));
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

impl Default for SnippetScraper {
    fn default() -> Self {
        Self::new()
    }
}
