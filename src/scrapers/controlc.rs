use super::traits::{Scraper, ScraperError, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;

/// ControlC scraper (controlc.com)
/// ControlC is a paste service that shows recent public pastes
pub struct ControlcScraper {
    base_url: String,
}

impl ControlcScraper {
    pub fn new() -> Self {
        ControlcScraper {
            base_url: "https://controlc.com".to_string(),
        }
    }

    pub fn with_url(url: String) -> Self {
        ControlcScraper { base_url: url }
    }
}

impl Default for ControlcScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for ControlcScraper {
    fn name(&self) -> &str {
        "controlc"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // ControlC has a recent pastes page
        let recent_url = format!("{}/recent", self.base_url);

        let response = client
            .get(&recent_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ScraperError::SourceUnavailable(format!(
                "ControlC returned {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let mut pastes = Vec::new();

        // Parse paste links from recent page
        // Format varies but typically: <a href="/PASTE_ID">
        // ControlC uses alphanumeric paste IDs
        let re = regex::Regex::new(r#"<a href="/([a-zA-Z0-9]{5,12})"[^>]*>"#).unwrap();

        for cap in re.captures_iter(&html).take(10) {
            let paste_id = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip common non-paste paths
            if paste_id.is_empty()
                || paste_id == "recent"
                || paste_id == "login"
                || paste_id == "register"
                || paste_id == "about"
                || paste_id == "privacy"
                || paste_id == "terms"
            {
                continue;
            }

            // Fetch paste content
            let paste_url = format!("{}/{}", self.base_url, paste_id);
            match client
                .get(&paste_url)
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                )
                .send()
                .await
            {
                Ok(content_response) => {
                    if content_response.status().is_success() {
                        if let Ok(page_html) = content_response.text().await {
                            // Extract content from the page
                            // Look for content in textarea or pre tags
                            let content = extract_paste_content(&page_html);

                            if !content.is_empty() && content.len() < 100000 {
                                let paste = DiscoveredPaste::new("controlc", paste_id, content)
                                    .with_url(paste_url)
                                    .with_title(format!("ControlC: {}", paste_id));
                                pastes.push(paste);
                                tracing::debug!("Fetched controlc paste: {}", paste_id);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch controlc paste {}: {}", paste_id, e);
                    continue;
                }
            }

            // Rate limit
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Ok(pastes)
    }
}

/// Extract paste content from HTML page
fn extract_paste_content(html: &str) -> String {
    // Try to find content in textarea
    if let Some(start) = html.find("<textarea") {
        if let Some(content_start) = html[start..].find('>') {
            let content_begin = start + content_start + 1;
            if let Some(end) = html[content_begin..].find("</textarea>") {
                let content = &html[content_begin..content_begin + end];
                return html_escape::decode_html_entities(content).to_string();
            }
        }
    }

    // Try to find content in pre tag with class "paste"
    if let Some(start) = html.find("<pre") {
        if let Some(content_start) = html[start..].find('>') {
            let content_begin = start + content_start + 1;
            if let Some(end) = html[content_begin..].find("</pre>") {
                let content = &html[content_begin..content_begin + end];
                return html_escape::decode_html_entities(content).to_string();
            }
        }
    }

    // Try div with id="paste_content" or similar
    let re = regex::Regex::new(r#"<div[^>]*id="paste[_-]?content"[^>]*>([\s\S]*?)</div>"#).ok();
    if let Some(regex) = re {
        if let Some(cap) = regex.captures(html) {
            if let Some(content) = cap.get(1) {
                return html_escape::decode_html_entities(content.as_str()).to_string();
            }
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controlc_scraper_creation() {
        let scraper = ControlcScraper::new();
        assert_eq!(scraper.name(), "controlc");
    }

    #[test]
    fn test_controlc_scraper_default() {
        let scraper = ControlcScraper::default();
        assert_eq!(scraper.name(), "controlc");
    }

    #[test]
    fn test_controlc_custom_url() {
        let custom_url = "https://ctrl.custom.com".to_string();
        let scraper = ControlcScraper::with_url(custom_url.clone());
        assert_eq!(scraper.base_url, custom_url);
    }

    #[test]
    fn test_extract_content_textarea() {
        let html = r#"<html><body><textarea class="paste">Hello World</textarea></body></html>"#;
        let content = extract_paste_content(html);
        assert_eq!(content, "Hello World");
    }

    #[test]
    fn test_extract_content_pre() {
        let html = r#"<html><body><pre class="code">Test Content</pre></body></html>"#;
        let content = extract_paste_content(html);
        assert_eq!(content, "Test Content");
    }
}
