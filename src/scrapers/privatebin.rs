use crate::models::DiscoveredPaste;
use crate::scrapers::traits::{Scraper, ScraperResult};
use async_trait::async_trait;
use base64::Engine;

pub struct PrivatebinScraper;

impl PrivatebinScraper {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Scraper for PrivatebinScraper {
    fn name(&self) -> &str {
        "privatebin"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // PrivateBin instances often have public directories or RSS feeds
        // Using privatebin.net public instance with directory endpoint
        let url = "https://privatebin.net/?dir";
        
        let response = client
            .get(url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::scrapers::ScraperError::Other(format!("HTTP {}", response.status())));
        }

        #[derive(serde::Deserialize)]
        struct DirItem {
            id: String,
            #[serde(default)]
            burnafterreading: bool,
        }

        let items: Vec<DirItem> = match response.json().await {
            Ok(v) => v,
            Err(_) => return Ok(vec![]), // No public directory available
        };

        let mut pastes = Vec::new();

        for item in items.into_iter().take(15) {
            // Skip burn-after-reading pastes
            if item.burnafterreading {
                continue;
            }

            // Fetch paste content via API
            let api_url = format!("https://privatebin.net/?pasteid={}", item.id);
            match client
                .get(&api_url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    #[derive(serde::Deserialize)]
                    struct PasteData {
                        #[serde(default)]
                        data: Option<String>,
                    }

                    if let Ok(paste_data) = resp.json::<PasteData>().await {
                        if let Some(content) = paste_data.data {
                            // PrivateBin uses client-side encryption, so we often get base64
                            // Try to decode if it looks like base64
                            let decoded = if content.chars().all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=') {
                                base64::engine::general_purpose::STANDARD.decode(&content)
                                    .ok()
                                    .and_then(|bytes| String::from_utf8(bytes).ok())
                            } else {
                                Some(content.clone())
                            };

                            if let Some(text) = decoded {
                                if !text.trim().is_empty() && text.len() < 500_000 {
                                    let paste = DiscoveredPaste::new("privatebin", &item.id, text)
                                        .with_url(format!("https://privatebin.net/?{}", item.id));
                                    pastes.push(paste);
                                }
                            }
                        }
                    }
                }
                _ => continue,
            }

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        Ok(pastes)
    }
}

impl Default for PrivatebinScraper {
    fn default() -> Self {
        Self::new()
    }
}
