use super::traits::{Scraper, ScraperError, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use serde::Deserialize;

/// psbdmp.ws scraper - indexes pastebin dumps with searchable credentials
/// Free API that searches for specific keywords like api_key, password, token, etc.
pub struct PsbdmpScraper {
    keywords: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PsbdmpResult {
    id: String,
    #[serde(default)]
    tags: String,
    #[serde(default)]
    length: i64,
    #[serde(default)]
    time: String,
    #[serde(default)]
    text: String,
}

impl PsbdmpScraper {
    pub fn new() -> Self {
        // Keywords to search for credential leaks
        PsbdmpScraper {
            keywords: vec![
                "apikey".to_string(),
                "api_key".to_string(),
                "password".to_string(),
                "secret_key".to_string(),
                "access_token".to_string(),
                "private_key".to_string(),
                "aws_key".to_string(),
                "discord_token".to_string(),
                "stripe_key".to_string(),
            ],
        }
    }
}

impl Default for PsbdmpScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for PsbdmpScraper {
    fn name(&self) -> &str {
        "psbdmp"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let mut pastes = Vec::new();
        
        // Rotate through keywords to get diverse results
        let keyword = &self.keywords[rand::random::<usize>() % self.keywords.len()];
        
        let url = format!("https://psbdmp.ws/api/search/{}", keyword);
        
        let response = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ScraperError::SourceUnavailable(format!(
                "psbdmp.ws returned {}",
                response.status()
            )));
        }

        let results: Vec<PsbdmpResult> = match response.json().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to parse psbdmp response: {}", e);
                return Ok(vec![]);
            }
        };

        // Take up to 20 results
        for result in results.into_iter().take(20) {
            // psbdmp returns pastebin IDs - fetch full content
            let paste_url = format!("https://pastebin.com/raw/{}", result.id);
            
            match client
                .get(&paste_url)
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                .send()
                .await
            {
                Ok(content_response) if content_response.status().is_success() => {
                    if let Ok(content) = content_response.text().await {
                        if !content.is_empty() && content.len() > 20 {
                            let title = if !result.text.is_empty() {
                                result.text.chars().take(50).collect::<String>()
                            } else {
                                format!("Leak: {}", keyword)
                            };
                            
                            let paste = DiscoveredPaste::new("psbdmp", &result.id, content)
                                .with_title(title)
                                .with_url(format!("https://pastebin.com/{}", result.id))
                                .with_syntax("plaintext".to_string());
                            
                            pastes.push(paste);
                        }
                    }
                }
                _ => continue,
            }
            
            // Small delay to avoid rate limits
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(pastes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psbdmp_scraper_creation() {
        let scraper = PsbdmpScraper::new();
        assert_eq!(scraper.name(), "psbdmp");
        assert!(!scraper.keywords.is_empty());
    }
}
