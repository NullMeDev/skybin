use super::credential_filter::contains_credentials;
use super::traits::{Scraper, ScraperError, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// Tor-based paste site scraper
/// Requires a SOCKS5 proxy (Tor) configured in config.toml
pub struct TorPastesScraper {
    onion_sites: Vec<OnionSite>,
    proxy_url: Option<String>,
}

struct OnionSite {
    name: &'static str,
    base_url: &'static str,
    recent_path: &'static str,
    raw_path: &'static str,
    id_pattern: &'static str,
}

impl TorPastesScraper {
    pub fn new() -> Self {
        TorPastesScraper {
            onion_sites: vec![
                OnionSite {
                    name: "stronghold",
                    base_url:
                        "http://strongerw2ise74v3duebgsvug4mehyhlpa7f6kfwnas7zofs3ber7yid.onion",
                    recent_path: "/paste/",
                    raw_path: "/paste/raw/",
                    id_pattern: r#"/paste/([a-zA-Z0-9]+)"#,
                },
                OnionSite {
                    name: "deepaste",
                    base_url: "http://depastedihrn3jtw.onion",
                    recent_path: "/last",
                    raw_path: "/show.php?md5=",
                    id_pattern: r#"md5=([a-f0-9]+)"#,
                },
            ],
            proxy_url: None,
        }
    }

    pub fn with_proxy(proxy: String) -> Self {
        let mut scraper = Self::new();
        if !proxy.is_empty() {
            scraper.proxy_url = Some(proxy);
        }
        scraper
    }
}

impl Default for TorPastesScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for TorPastesScraper {
    fn name(&self) -> &str {
        "tor_pastes"
    }

    async fn fetch_recent(&self, _client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // If no proxy configured, skip silently
        let proxy_url = match &self.proxy_url {
            Some(p) if !p.is_empty() => p.clone(),
            _ => {
                tracing::debug!("Tor scraper skipped - no SOCKS5 proxy configured");
                return Ok(vec![]);
            }
        };

        // Build client with Tor proxy
        let proxy = match reqwest::Proxy::all(&proxy_url) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Invalid proxy URL: {}", e);
                return Ok(vec![]);
            }
        };

        let tor_client = match reqwest::Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(60))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to build Tor client: {}", e);
                return Ok(vec![]);
            }
        };

        let mut pastes = Vec::new();

        for site in &self.onion_sites {
            match self.scrape_onion_site(&tor_client, site).await {
                Ok(mut site_pastes) => {
                    pastes.append(&mut site_pastes);
                }
                Err(e) => {
                    tracing::warn!("Failed to scrape {}: {}", site.name, e);
                }
            }
        }

        Ok(pastes)
    }
}

impl TorPastesScraper {
    async fn scrape_onion_site(
        &self,
        client: &reqwest::Client,
        site: &OnionSite,
    ) -> ScraperResult<Vec<DiscoveredPaste>> {
        let mut pastes = Vec::new();
        let url = format!("{}{}", site.base_url, site.recent_path);

        let response = client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; rv:109.0) Gecko/20100101 Firefox/115.0",
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ScraperError::SourceUnavailable(format!(
                "{} returned {}",
                site.name,
                response.status()
            )));
        }

        let html = response.text().await?;
        let id_regex = regex::Regex::new(site.id_pattern).unwrap();

        for cap in id_regex.captures_iter(&html).take(15) {
            if let Some(id) = cap.get(1) {
                let paste_id = id.as_str();
                let raw_url = format!("{}{}{}", site.base_url, site.raw_path, paste_id);

                match client
                    .get(&raw_url)
                    .header("User-Agent", "Mozilla/5.0")
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(content) = resp.text().await {
                            // Only keep pastes with credentials
                            if !content.is_empty() && contains_credentials(&content) {
                                let title =
                                    content.lines().next().unwrap_or("Tor Paste").to_string();
                                let paste = DiscoveredPaste::new("tor_pastes", paste_id, content)
                                    .with_title(title.chars().take(50).collect::<String>())
                                    .with_url(format!("{}/paste/{}", site.base_url, paste_id))
                                    .with_syntax("plaintext".to_string());
                                pastes.push(paste);
                            }
                        }
                    }
                    _ => continue,
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }

        Ok(pastes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tor_scraper_creation() {
        let scraper = TorPastesScraper::new();
        assert_eq!(scraper.name(), "tor_pastes");
    }

    #[test]
    fn test_tor_scraper_with_proxy() {
        let scraper = TorPastesScraper::with_proxy("socks5://127.0.0.1:9050".to_string());
        assert!(scraper.proxy_url.is_some());
    }
}
