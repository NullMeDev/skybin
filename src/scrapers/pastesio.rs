use super::traits::{Scraper, ScraperError, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use serde::Deserialize;

/// pastesio.com scraper - modern Pastebin alternative with public API
/// API docs indicate they have a public pastes endpoint
pub struct PastesioScraper {
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct PastesioResponse {
    #[serde(default)]
    data: Vec<PastesioItem>,
}

#[derive(Debug, Deserialize)]
struct PastesioItem {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    syntax: String,
    #[serde(default)]
    created_at: String,
}

impl PastesioScraper {
    pub fn new() -> Self {
        PastesioScraper {
            base_url: "https://pastesio.com".to_string(),
        }
    }
}

impl Default for PastesioScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for PastesioScraper {
    fn name(&self) -> &str {
        "pastesio"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let mut pastes = Vec::new();
        
        // Try the archive/recent page first
        let archive_url = format!("{}/archive", self.base_url);
        
        let response = match client
            .get(&archive_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Accept", "text/html,application/xhtml+xml")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Err(ScraperError::SourceUnavailable(format!(
                    "pastesio.com connection failed: {}",
                    e
                )));
            }
        };

        if !response.status().is_success() {
            return Err(ScraperError::SourceUnavailable(format!(
                "pastesio.com returned {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        
        // Parse paste IDs from archive page
        // Looking for links like /paste/abc123 or /raw/abc123
        let paste_regex = regex::Regex::new(r#"href="/(paste|raw|view)/([a-zA-Z0-9]+)"#).unwrap();
        
        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        
        for cap in paste_regex.captures_iter(&html) {
            let paste_id = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            
            if paste_id.is_empty() || seen_ids.contains(&paste_id) {
                continue;
            }
            seen_ids.insert(paste_id.clone());
            
            if seen_ids.len() > 15 {
                break;
            }
            
            // Fetch raw content
            let raw_url = format!("{}/raw/{}", self.base_url, paste_id);
            
            match client
                .get(&raw_url)
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                .send()
                .await
            {
                Ok(content_resp) if content_resp.status().is_success() => {
                    if let Ok(content) = content_resp.text().await {
                        if !content.is_empty() && content.len() > 20 {
                            let title = format!("pastesio:{}", &paste_id);
                            
                            let paste = DiscoveredPaste::new("pastesio", &paste_id, content)
                                .with_title(title)
                                .with_url(format!("{}/paste/{}", self.base_url, paste_id))
                                .with_syntax("plaintext".to_string());
                            
                            pastes.push(paste);
                        }
                    }
                }
                _ => continue,
            }
            
            // Rate limit between fetches
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }
        
        Ok(pastes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pastesio_scraper_creation() {
        let scraper = PastesioScraper::new();
        assert_eq!(scraper.name(), "pastesio");
    }
}
