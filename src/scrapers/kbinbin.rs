use crate::models::DiscoveredPaste;
use crate::scrapers::traits::{Scraper, ScraperResult};
use async_trait::async_trait;
use regex::Regex;
use once_cell::sync::Lazy;

pub struct KbinbinScraper;

static PASTE_ID_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"href="(/[a-zA-Z0-9]{6,12})""#).unwrap());

impl KbinbinScraper {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Scraper for KbinbinScraper {
    fn name(&self) -> &str {
        "kbinbin"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Kbinbin (kbinbin.com) - paste aggregator with recent list
        let url = "https://kbinbin.com/recent";
        
        let response = client
            .get(url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::scrapers::ScraperError::Other(format!("HTTP {}", response.status())));
        }

        let html = response.text().await?;
        let mut pastes = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Extract paste IDs from HTML
        for cap in PASTE_ID_REGEX.captures_iter(&html) {
            if let Some(id_match) = cap.get(1) {
                let id = id_match.as_str().trim_start_matches('/');
                if !seen_ids.insert(id.to_string()) {
                    continue;
                }

                // Fetch raw content
                let raw_url = format!("https://kbinbin.com/{}/raw", id);
                match client
                    .get(&raw_url)
                    .timeout(std::time::Duration::from_secs(10))
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(content) = resp.text().await {
                            if !content.trim().is_empty() && content.len() < 500_000 {
                                let paste = DiscoveredPaste::new("kbinbin", id, content)
                                    .with_url(format!("https://kbinbin.com/{}", id));
                                pastes.push(paste);
                            }
                        }
                    }
                    _ => continue,
                }

                if pastes.len() >= 15 {
                    break;
                }

                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }
        }

        Ok(pastes)
    }
}

impl Default for KbinbinScraper {
    fn default() -> Self {
        Self::new()
    }
}
