use crate::models::DiscoveredPaste;
use crate::scrapers::traits::{Scraper, ScraperResult};
use async_trait::async_trait;
use regex::Regex;
use once_cell::sync::Lazy;

pub struct ZeroBinScraper;

static PASTE_LINK_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"href="(/paste/[a-zA-Z0-9]+)""#).unwrap());

impl ZeroBinScraper {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Scraper for ZeroBinScraper {
    fn name(&self) -> &str {
        "zerobin"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // 0bin (zerobin.net) - encrypted pastebin, limited public discovery
        // Try to fetch from known public instances
        let instances = vec![
            "https://0bin.net",
            "https://zerobin.net",
        ];

        let mut pastes = Vec::new();

        for base_url in instances {
            // Try recent/list endpoint if available
            let list_url = format!("{}/list", base_url);
            let response = match client
                .get(&list_url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };

            let html = match response.text().await {
                Ok(h) => h,
                Err(_) => continue,
            };

            let mut seen_ids = std::collections::HashSet::new();

            // Extract paste IDs from HTML
            for cap in PASTE_LINK_REGEX.captures_iter(&html) {
                if let Some(path_match) = cap.get(1) {
                    let path = path_match.as_str();
                    let id = path.trim_start_matches("/paste/");
                    
                    if !seen_ids.insert(id.to_string()) {
                        continue;
                    }

                    // Fetch paste content
                    let paste_url = format!("{}{}", base_url, path);
                    match client
                        .get(&paste_url)
                        .timeout(std::time::Duration::from_secs(10))
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status().is_success() => {
                            // 0bin typically returns JSON with encrypted data
                            #[derive(serde::Deserialize)]
                            struct ZeroBinPaste {
                                #[serde(default)]
                                content: Option<String>,
                                #[serde(default)]
                                paste: Option<String>,
                            }

                            if let Ok(data) = resp.json::<ZeroBinPaste>().await {
                                let text = data.content.or(data.paste);
                                if let Some(content) = text {
                                    if !content.trim().is_empty() && content.len() < 500_000 {
                                        let paste = DiscoveredPaste::new("zerobin", id, content)
                                            .with_url(paste_url);
                                        pastes.push(paste);
                                    }
                                }
                            }
                        }
                        _ => continue,
                    }

                    if pastes.len() >= 10 {
                        break;
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                }
            }

            if pastes.len() >= 10 {
                break;
            }
        }

        Ok(pastes)
    }
}

impl Default for ZeroBinScraper {
    fn default() -> Self {
        Self::new()
    }
}
