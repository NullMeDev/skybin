use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct GistOwner {
    login: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GistFile {
    filename: String,
    #[serde(default)]
    content: String,
    language: Option<String>,
    raw_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Gist {
    id: String,
    url: String,
    #[serde(default)]
    description: Option<String>,
    owner: GistOwner,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    files: std::collections::HashMap<String, GistFile>,
    #[serde(default)]
    public: bool,
}

/// GitHub Gists scraper using the public API
/// Fetches recently updated public gists
pub struct GitHubGistsScraper {
    api_url: String,
    github_token: Option<String>,
}

impl GitHubGistsScraper {
    pub fn new() -> Self {
        GitHubGistsScraper {
            api_url: "https://api.github.com/gists/public".to_string(),
            github_token: None,
        }
    }

    pub fn with_token(token: String) -> Self {
        GitHubGistsScraper {
            api_url: "https://api.github.com/gists/public".to_string(),
            github_token: Some(token),
        }
    }

    pub fn with_url(url: String) -> Self {
        GitHubGistsScraper {
            api_url: url,
            github_token: None,
        }
    }

    pub fn with_url_and_token(url: String, token: String) -> Self {
        GitHubGistsScraper {
            api_url: url,
            github_token: Some(token),
        }
    }
}

impl Default for GitHubGistsScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for GitHubGistsScraper {
    fn name(&self) -> &str {
        "gists"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let mut request = client
            .get(&self.api_url)
            .query(&[("per_page", "30"), ("sort", "updated")]);

        // Add authentication header if token is provided (increases rate limit)
        if let Some(token) = &self.github_token {
            request = request.header("Authorization", format!("token {}", token));
        }

        request = request.header(
            "User-Agent",
            "SkyBin-Gist-Scraper/1.0 (anonymous content aggregator)",
        );

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(crate::scrapers::ScraperError::SourceUnavailable(format!(
                "GitHub API returned {}",
                response.status()
            )));
        }

        let gists: Vec<Gist> = response.json().await?;

        let mut pastes = Vec::new();

        for gist in gists {
            // Only process public gists
            if !gist.public {
                continue;
            }

            // Get the first file (gists can have multiple files, we take primary one)
            if let Some((filename, file)) = gist.files.iter().next() {
                // Skip empty files
                if file.content.is_empty() {
                    continue;
                }

                let paste = DiscoveredPaste::new("gists", &gist.id, file.content.clone())
                    .with_title(
                        gist.description
                            .unwrap_or_else(|| format!("Gist: {}", filename)),
                    )
                    // Note: author is not set here - it will be None before storage
                    // This is intentional per anonymization requirements
                    .with_url(gist.url)
                    .with_syntax(file.language.clone().unwrap_or_else(|| "text".to_string()))
                    .with_created_at(
                        chrono::DateTime::parse_from_rfc3339(&gist.created_at)
                            .map(|dt| dt.timestamp())
                            .unwrap_or(0),
                    );

                pastes.push(paste);
            }
        }

        Ok(pastes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_gists_scraper_creation() {
        let scraper = GitHubGistsScraper::new();
        assert_eq!(scraper.name(), "gists");
        assert!(scraper.github_token.is_none());
    }

    #[test]
    fn test_github_gists_scraper_with_token() {
        let token = "test_token_123".to_string();
        let scraper = GitHubGistsScraper::with_token(token.clone());
        assert_eq!(scraper.name(), "gists");
        assert_eq!(scraper.github_token, Some(token));
    }

    #[test]
    fn test_github_gists_scraper_default() {
        let scraper = GitHubGistsScraper::default();
        assert_eq!(scraper.name(), "gists");
    }

    #[test]
    fn test_github_gists_custom_url() {
        let custom_url = "https://custom.github.api/gists/public".to_string();
        let scraper = GitHubGistsScraper::with_url(custom_url.clone());
        assert_eq!(scraper.api_url, custom_url);
    }

    #[test]
    fn test_github_gists_custom_url_and_token() {
        let url = "https://custom.github.api/gists".to_string();
        let token = "test_token".to_string();
        let scraper = GitHubGistsScraper::with_url_and_token(url.clone(), token.clone());
        assert_eq!(scraper.api_url, url);
        assert_eq!(scraper.github_token, Some(token));
    }
}
