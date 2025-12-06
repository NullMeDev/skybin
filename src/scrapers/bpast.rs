use super::traits::{Scraper, ScraperError, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// bpa.st scraper - simple paste site with recent pastes
pub struct BpastScraper {
    base_url: String,
}

impl BpastScraper {
    pub fn new() -> Self {
        BpastScraper {
            base_url: "https://bpa.st".to_string(),
        }
    }
}

impl Default for BpastScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for BpastScraper {
    fn name(&self) -> &str {
        "bpast"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let mut pastes = Vec::new();

        // bpa.st has a recent page we can scrape
        let response = match client
            .get(&self.base_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Err(ScraperError::SourceUnavailable(format!(
                    "bpa.st connection failed: {}",
                    e
                )));
            }
        };

        if !response.status().is_success() {
            return Err(ScraperError::SourceUnavailable(format!(
                "bpa.st returned {}",
                response.status()
            )));
        }

        let html = response.text().await?;

        // Parse paste IDs - bpa.st uses short alphanumeric IDs
        // Links like href="/abcd" or href="/raw/abcd"
        let paste_regex = regex::Regex::new(r#"href="/([a-zA-Z0-9]{4,8})"#).unwrap();

        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for cap in paste_regex.captures_iter(&html) {
            let paste_id = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            // Skip common paths that aren't pastes
            if paste_id.is_empty()
                || seen_ids.contains(&paste_id)
                || paste_id == "about"
                || paste_id == "api"
                || paste_id == "static"
                || paste_id == "raw"
            {
                continue;
            }
            seen_ids.insert(paste_id.clone());

            if seen_ids.len() > 10 {
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
                            let title = format!("bpa.st:{}", &paste_id);

                            let paste = DiscoveredPaste::new("bpast", &paste_id, content)
                                .with_title(title)
                                .with_url(format!("{}/{}", self.base_url, paste_id))
                                .with_syntax("plaintext".to_string());

                            pastes.push(paste);
                        }
                    }
                }
                _ => continue,
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        Ok(pastes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpast_scraper_creation() {
        let scraper = BpastScraper::new();
        assert_eq!(scraper.name(), "bpast");
    }
}
