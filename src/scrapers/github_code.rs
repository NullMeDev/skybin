use super::credential_filter::contains_credentials;
use super::traits::{Scraper, ScraperResult};
use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use serde::Deserialize;

/// Search queries targeting exposed secrets
/// These are rotated to spread rate limit usage
const SECRET_QUERIES: &[&str] = &[
    // AWS
    r#"AKIA extension:env"#,
    r#"AKIA extension:json"#,
    r#"aws_secret_access_key extension:yaml"#,
    r#"aws_secret_access_key extension:env"#,
    // OpenAI
    r#"sk-proj- extension:env"#,
    r#""sk-" openai extension:py"#,
    // Database strings
    r#"mongodb+srv:// password extension:env"#,
    r#"postgres:// password extension:env"#,
    r#"mysql:// password extension:yaml"#,
    // Private keys
    r#""BEGIN RSA PRIVATE KEY" extension:pem"#,
    r#""BEGIN OPENSSH PRIVATE KEY" extension:txt"#,
    r#""BEGIN EC PRIVATE KEY""#,
    // API tokens
    r#"ghp_ extension:json"#,
    r#"github_pat_ extension:env"#,
    r#"xoxb- extension:env"#,
    r#"xoxp- slack extension:yaml"#,
    // Stripe (live keys only)
    r#"sk_live_ extension:env"#,
    r#"sk_live_ extension:json"#,
    // Discord
    r#"discord token extension:env"#,
    r#""Bot " discord extension:py"#,
    // Firebase/Google
    r#"AIzaSy extension:json"#,
    r#"firebase apiKey extension:js"#,
    // Twilio
    r#"twilio_auth_token extension:env"#,
    r#"TWILIO_AUTH_TOKEN extension:yaml"#,
    // SendGrid
    r#"SG. sendgrid extension:env"#,
    // JWT secrets
    r#"JWT_SECRET extension:env"#,
    r#"jwt_secret extension:yaml"#,
    // Generic patterns
    r#"password= extension:env"#,
    r#"api_key= extension:env"#,
    r#"secret_key= extension:env"#,
    r#"access_token= extension:env"#,
    // Telegram bots
    r#"telegram bot token extension:env"#,
    r#"TELEGRAM_TOKEN extension:env"#,
];

#[derive(Debug, Deserialize)]
struct CodeSearchResponse {
    total_count: i64,
    incomplete_results: bool,
    items: Vec<CodeSearchItem>,
}

#[derive(Debug, Deserialize)]
struct CodeSearchItem {
    name: String,
    path: String,
    sha: String,
    url: String,
    html_url: String,
    repository: Repository,
    #[serde(default)]
    text_matches: Vec<TextMatch>,
}

#[derive(Debug, Deserialize)]
struct Repository {
    id: i64,
    full_name: String,
    html_url: String,
    #[serde(default)]
    private: bool,
}

#[derive(Debug, Deserialize)]
struct TextMatch {
    fragment: String,
    #[serde(default)]
    matches: Vec<Match>,
}

#[derive(Debug, Deserialize)]
struct Match {
    text: String,
    #[serde(default)]
    indices: Vec<i32>,
}

/// GitHub Code Search scraper - searches for exposed secrets in public repos
pub struct GitHubCodeScraper {
    github_token: Option<String>,
    query_index: std::sync::atomic::AtomicUsize,
}

impl GitHubCodeScraper {
    pub fn new() -> Self {
        GitHubCodeScraper {
            github_token: None,
            query_index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn with_token(token: String) -> Self {
        GitHubCodeScraper {
            github_token: Some(token),
            query_index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Get next search query (rotates through list)
    fn next_query(&self) -> &'static str {
        let idx = self
            .query_index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        SECRET_QUERIES[idx % SECRET_QUERIES.len()]
    }

    /// Fetch raw file content from GitHub
    async fn fetch_raw_content(
        &self,
        client: &reqwest::Client,
        item: &CodeSearchItem,
    ) -> Option<String> {
        // Convert html_url to raw content URL
        // https://github.com/user/repo/blob/main/file.txt ->
        // https://raw.githubusercontent.com/user/repo/main/file.txt
        let raw_url = item
            .html_url
            .replace("github.com", "raw.githubusercontent.com")
            .replace("/blob/", "/");

        let mut request = client
            .get(&raw_url)
            .header("User-Agent", "SkyBin-CodeSearch/1.0");

        if let Some(token) = &self.github_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        match request.send().await {
            Ok(resp) if resp.status().is_success() => {
                let content = resp.text().await.ok()?;
                // Limit content size
                if content.len() > 100_000 {
                    Some(content[..100_000].to_string())
                } else {
                    Some(content)
                }
            }
            _ => None,
        }
    }

    /// Extract context around the secret for title generation
    fn extract_secret_type(content: &str) -> &'static str {
        let lower = content.to_lowercase();

        if content.contains("AKIA") || lower.contains("aws_secret") {
            "AWS Credentials"
        } else if content.contains("sk-proj-") || content.contains("sk-") && lower.contains("openai") {
            "OpenAI API Key"
        } else if content.contains("ghp_") || content.contains("github_pat_") {
            "GitHub Token"
        } else if content.contains("sk_live_") {
            "Stripe Live Key"
        } else if lower.contains("mongodb") || lower.contains("postgres") || lower.contains("mysql") {
            "Database Credentials"
        } else if content.contains("BEGIN") && content.contains("PRIVATE KEY") {
            "Private Key"
        } else if content.contains("xoxb-") || content.contains("xoxp-") {
            "Slack Token"
        } else if content.contains("AIzaSy") {
            "Google/Firebase Key"
        } else if lower.contains("discord") && lower.contains("token") {
            "Discord Token"
        } else if content.contains("SG.") && lower.contains("sendgrid") {
            "SendGrid API Key"
        } else if lower.contains("twilio") {
            "Twilio Credentials"
        } else if lower.contains("jwt") && lower.contains("secret") {
            "JWT Secret"
        } else if lower.contains("telegram") && lower.contains("token") {
            "Telegram Bot Token"
        } else {
            "Exposed Secret"
        }
    }
}

impl Default for GitHubCodeScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for GitHubCodeScraper {
    fn name(&self) -> &str {
        "github"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Use rotating queries to find different types of secrets
        let query = self.next_query();

        let mut request = client
            .get("https://api.github.com/search/code")
            .query(&[
                ("q", query),
                ("per_page", "10"),
                ("sort", "indexed"),
                ("order", "desc"),
            ])
            // Request text match fragments
            .header("Accept", "application/vnd.github.text-match+json")
            .header("User-Agent", "SkyBin-CodeSearch/1.0");

        if let Some(token) = &self.github_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        // Handle rate limiting
        if response.status() == reqwest::StatusCode::FORBIDDEN
            || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            return Err(crate::scrapers::ScraperError::RateLimited);
        }

        if !response.status().is_success() {
            return Err(crate::scrapers::ScraperError::SourceUnavailable(format!(
                "GitHub API returned {}",
                response.status()
            )));
        }

        let search_result: CodeSearchResponse = response.json().await?;

        let mut pastes = Vec::new();

        for item in search_result.items.iter().take(10) {
            // Skip private repos (shouldn't happen in search results, but safety check)
            if item.repository.private {
                continue;
            }

            // Fetch full file content
            let content = match self.fetch_raw_content(client, item).await {
                Some(c) => c,
                None => {
                    // Fall back to text match fragments if raw fetch fails
                    if !item.text_matches.is_empty() {
                        item.text_matches
                            .iter()
                            .map(|tm| tm.fragment.clone())
                            .collect::<Vec<_>>()
                            .join("\n---\n")
                    } else {
                        continue;
                    }
                }
            };

            // Verify it contains credentials
            if !contains_credentials(&content) {
                continue;
            }

            // Generate descriptive title
            let secret_type = Self::extract_secret_type(&content);
            let title = format!(
                "[GH] {} in {}/{}",
                secret_type, item.repository.full_name, item.name
            );

            // Create unique ID from repo+path+sha
            let source_id = format!("{}:{}:{}", item.repository.id, item.path, &item.sha[..8]);

            let paste = DiscoveredPaste::new("github", &source_id, content)
                .with_title(title)
                .with_url(item.html_url.clone())
                .with_syntax(
                    item.name
                        .rsplit('.')
                        .next()
                        .unwrap_or("text")
                        .to_string(),
                );

            pastes.push(paste);

            // Rate limit between fetches
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(pastes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_code_scraper_creation() {
        let scraper = GitHubCodeScraper::new();
        assert_eq!(scraper.name(), "github");
        assert!(scraper.github_token.is_none());
    }

    #[test]
    fn test_github_code_scraper_with_token() {
        let token = "ghp_test123".to_string();
        let scraper = GitHubCodeScraper::with_token(token.clone());
        assert_eq!(scraper.github_token, Some(token));
    }

    #[test]
    fn test_query_rotation() {
        let scraper = GitHubCodeScraper::new();
        let q1 = scraper.next_query();
        let q2 = scraper.next_query();
        let q3 = scraper.next_query();
        // Should be different queries (rotating through list)
        assert_ne!(q1, q2);
        assert_ne!(q2, q3);
    }

    #[test]
    fn test_secret_type_detection() {
        assert_eq!(
            GitHubCodeScraper::extract_secret_type("AKIAIOSFODNN7EXAMPLE"),
            "AWS Credentials"
        );
        assert_eq!(
            GitHubCodeScraper::extract_secret_type("sk-proj-abc123"),
            "OpenAI API Key"
        );
        assert_eq!(
            GitHubCodeScraper::extract_secret_type("ghp_xxxxxxxxxxxx"),
            "GitHub Token"
        );
        assert_eq!(
            GitHubCodeScraper::extract_secret_type("sk_live_xxxxx"),
            "Stripe Live Key"
        );
        assert_eq!(
            GitHubCodeScraper::extract_secret_type("mongodb+srv://user:pass@host"),
            "Database Credentials"
        );
        assert_eq!(
            GitHubCodeScraper::extract_secret_type("-----BEGIN RSA PRIVATE KEY-----"),
            "Private Key"
        );
    }
}
