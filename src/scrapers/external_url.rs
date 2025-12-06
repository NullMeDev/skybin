use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// External URL scraper - monitors a queue of submitted paste URLs
/// This allows users to submit paste URLs from any source for monitoring
#[derive(Clone)]
pub struct ExternalUrlScraper {
    url_queue: Arc<Mutex<VecDeque<String>>>,
}

impl ExternalUrlScraper {
    pub fn new() -> Self {
        ExternalUrlScraper {
            url_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Add a URL to the monitoring queue
    pub fn add_url(&self, url: String) {
        if let Ok(mut queue) = self.url_queue.lock() {
            if !queue.contains(&url) {
                queue.push_back(url);
            }
        }
    }

    /// Add multiple URLs to the queue
    pub fn add_urls(&self, urls: Vec<String>) {
        if let Ok(mut queue) = self.url_queue.lock() {
            for url in urls {
                if !queue.contains(&url) {
                    queue.push_back(url);
                }
            }
        }
    }

    /// Get queue size
    pub fn queue_size(&self) -> usize {
        self.url_queue.lock().map(|q| q.len()).unwrap_or(0)
    }
}

impl Default for ExternalUrlScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for ExternalUrlScraper {
    fn name(&self) -> &str {
        "external_url"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let mut pastes = Vec::new();

        // Process up to 10 URLs from the queue per scrape
        let urls_to_process: Vec<String> = {
            let mut queue = self.url_queue.lock().unwrap();
            (0..10).filter_map(|_| queue.pop_front()).collect()
        };

        if urls_to_process.is_empty() {
            return Ok(pastes);
        }

        tracing::info!("Processing {} submitted URLs", urls_to_process.len());

        for url in urls_to_process {
            // Determine source from URL
            let source = if url.contains("pastebin.com") {
                "pastebin"
            } else if url.contains("gist.github.com") {
                "gist"
            } else if url.contains("paste.ee") {
                "paste_ee"
            } else if url.contains("dpaste.") {
                "dpaste"
            } else if url.contains("rentry.") {
                "rentry"
            } else if url.contains("hastebin") {
                "hastebin"
            } else {
                "external"
            };

            // Fetch the content
            match client
                .get(&url)
                .header("User-Agent", "SkyBin/2.1.0 (security research)")
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(content) = response.text().await {
                            // Extract ID from URL (simple approach)
                            let id = url
                                .split('/')
                                .next_back()
                                .unwrap_or("unknown")
                                .split('?')
                                .next()
                                .unwrap_or("unknown")
                                .to_string();

                            let paste = DiscoveredPaste::new(source, &id, content)
                                .with_url(url.clone())
                                .with_title(format!("External: {}", id));

                            pastes.push(paste);
                            tracing::info!("✓ Fetched external paste from {}", url);
                        }
                    } else {
                        tracing::warn!("Failed to fetch {}: {}", url, response.status());
                    }
                }
                Err(e) => {
                    tracing::error!("Error fetching {}: {}", url, e);
                }
            }
        }

        Ok(pastes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_url_scraper_creation() {
        let scraper = ExternalUrlScraper::new();
        assert_eq!(scraper.name(), "external_url");
        assert_eq!(scraper.queue_size(), 0);
    }

    #[test]
    fn test_add_url() {
        let scraper = ExternalUrlScraper::new();
        scraper.add_url("https://pastebin.com/test123".to_string());
        assert_eq!(scraper.queue_size(), 1);
    }

    #[test]
    fn test_add_urls() {
        let scraper = ExternalUrlScraper::new();
        scraper.add_urls(vec![
            "https://pastebin.com/test1".to_string(),
            "https://pastebin.com/test2".to_string(),
        ]);
        assert_eq!(scraper.queue_size(), 2);
    }

    #[test]
    fn test_no_duplicates() {
        let scraper = ExternalUrlScraper::new();
        scraper.add_url("https://pastebin.com/test".to_string());
        scraper.add_url("https://pastebin.com/test".to_string());
        assert_eq!(scraper.queue_size(), 1);
    }
}
