use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use super::traits::{Scraper, ScraperResult};

pub struct IdeoneScraper;

impl IdeoneScraper {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IdeoneScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for IdeoneScraper {
    fn name(&self) -> &str {
        "ideone"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let resp = client
            .get("https://ideone.com/recent")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;
        
        let html = resp.text().await?;
        let mut pastes = Vec::new();
        let re = regex::Regex::new(r#"href="/([a-zA-Z0-9]{6})""#).unwrap();
        
        for cap in re.captures_iter(&html).take(15) {
            if let Some(id) = cap.get(1) {
                let paste_id = id.as_str();
                let raw_url = format!("https://ideone.com/plain/{}", paste_id);
                if let Ok(content_resp) = client.get(&raw_url)
                    .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                    .send()
                    .await
                {
                    if let Ok(content) = content_resp.text().await {
                        if !content.is_empty() && content.len() < 100000 {
                            pastes.push(DiscoveredPaste::new("ideone", paste_id, content)
                                .with_url(format!("https://ideone.com/{}", paste_id)));
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
            }
        }
        
        Ok(pastes)
    }
}
