use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

pub struct Paste2Scraper;

impl Paste2Scraper {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Paste2Scraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for Paste2Scraper {
    fn name(&self) -> &str {
        "paste2"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let resp = client
            .get("https://paste2.org/")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        let html = resp.text().await?;
        let mut pastes = Vec::new();
        let re = regex::Regex::new(r#"href="/([a-zA-Z0-9]{8,12})""#).unwrap();

        for cap in re.captures_iter(&html).take(10) {
            if let Some(id) = cap.get(1) {
                let paste_id = id.as_str();
                if paste_id.len() >= 8 {
                    let raw_url = format!("https://paste2.org/{}/raw", paste_id);
                    if let Ok(content_resp) = client
                        .get(&raw_url)
                        .header(
                            "User-Agent",
                            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                        )
                        .send()
                        .await
                    {
                        if let Ok(content) = content_resp.text().await {
                            if !content.is_empty() && content.len() > 50 && content.len() < 100000 {
                                pastes.push(
                                    DiscoveredPaste::new("paste2", paste_id, content)
                                        .with_url(format!("https://paste2.org/{}", paste_id)),
                                );
                            }
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                }
            }
        }

        Ok(pastes)
    }
}
