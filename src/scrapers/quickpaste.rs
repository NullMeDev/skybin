use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use super::traits::{Scraper, ScraperResult};

pub struct QuickpasteScraper;

impl QuickpasteScraper {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuickpasteScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for QuickpasteScraper {
    fn name(&self) -> &str {
        "quickpaste"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let resp = client
            .get("https://quickpaste.net/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;
        
        let html = resp.text().await?;
        let mut pastes = Vec::new();
        let re = regex::Regex::new(r#"href="/([a-zA-Z0-9]{5,10})""#).unwrap();
        
        for cap in re.captures_iter(&html).take(10) {
            if let Some(id) = cap.get(1) {
                let paste_id = id.as_str();
                // Skip common navigation links
                if paste_id == "about" || paste_id == "api" || paste_id == "terms" {
                    continue;
                }
                let raw_url = format!("https://quickpaste.net/{}/raw", paste_id);
                if let Ok(content_resp) = client.get(&raw_url)
                    .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                    .send()
                    .await
                {
                    if let Ok(content) = content_resp.text().await {
                        if !content.is_empty() && content.len() > 50 && content.len() < 100000 {
                            pastes.push(DiscoveredPaste::new("quickpaste", paste_id, content)
                                .with_url(format!("https://quickpaste.net/{}", paste_id)));
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }
        }
        
        Ok(pastes)
    }
}
