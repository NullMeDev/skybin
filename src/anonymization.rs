/// Anonymization utilities for protecting user privacy
/// 
/// This module ensures that all data stored in SkyBin is completely anonymous:
/// - User-submitted pastes: author names are stripped
/// - Scraped pastes: author names and URLs are anonymized
/// - No IP addresses or user identifiers are stored
/// - No personal information is retained

use crate::models::DiscoveredPaste;

/// Configuration for anonymization behavior
#[derive(Debug, Clone)]
pub struct AnonymizationConfig {
    /// Whether to strip author names from pastes
    pub strip_authors: bool,
    /// Whether to strip URLs from pastes
    pub strip_urls: bool,
    /// Whether to sanitize titles (remove potentially identifying information)
    pub sanitize_titles: bool,
}

impl Default for AnonymizationConfig {
    fn default() -> Self {
        Self {
            strip_authors: true,
            strip_urls: true,
            sanitize_titles: true,
        }
    }
}

/// Anonymize a discovered paste before storing
pub fn anonymize_discovered_paste(
    mut paste: DiscoveredPaste,
    config: &AnonymizationConfig,
) -> DiscoveredPaste {
    if config.strip_authors {
        paste.author = None;
    }
    
    if config.strip_urls {
        paste.url = String::new();
    }
    
    if config.sanitize_titles {
        // Remove email addresses and potential identifiers from titles
        if let Some(title) = paste.title {
            paste.title = Some(sanitize_title(&title));
        }
    }
    
    paste
}

/// Sanitize a title to remove potentially identifying information
fn sanitize_title(title: &str) -> String {
    let mut sanitized = title.to_string();
    
    // Remove email addresses
    sanitized = remove_emails(&sanitized);
    
    // Remove URLs
    sanitized = remove_urls(&sanitized);
    
    // Remove usernames (common patterns)
    sanitized = remove_usernames(&sanitized);
    
    sanitized.trim().to_string()
}

/// Remove email addresses from text
fn remove_emails(text: &str) -> String {
    // Simple email pattern removal
    let re = regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
        .unwrap_or_else(|_| regex::Regex::new(r"(.+)").unwrap());
    re.replace_all(text, "[redacted@email]").to_string()
}

/// Remove URLs from text
fn remove_urls(text: &str) -> String {
    // Simple URL pattern removal
    let re = regex::Regex::new(r"https?://[^\s]+")
        .unwrap_or_else(|_| regex::Regex::new(r"(.+)").unwrap());
    re.replace_all(text, "[redacted-url]").to_string()
}

/// Remove common username patterns
fn remove_usernames(text: &str) -> String {
    let mut result = text.to_string();
    
    // Remove @username patterns (common on Twitter, GitHub, etc)
    let re = regex::Regex::new(r"@[a-zA-Z0-9_-]+")
        .unwrap_or_else(|_| regex::Regex::new(r"(.+)").unwrap());
    result = re.replace_all(&result, "[user]").to_string();
    
    result
}

/// Create a privacy-safe author display (if needed)
/// Returns None if author should not be displayed
pub fn get_safe_author(_author: Option<&str>) -> Option<String> {
    // Authors are completely anonymized - no display needed
    None
}

/// Verify that a paste contains no sensitive PII
pub fn verify_anonymity(title: Option<&str>, author: Option<&str>) -> bool {
    // Author must be None (stripped)
    if author.is_some() {
        return false;
    }
    
    // Title should not contain obvious PII
    if let Some(t) = title {
        // Check for email patterns
        if t.contains("@") && t.contains(".") {
            return false;
        }
        // Check for http/https
        if t.contains("http://") || t.contains("https://") {
            return false;
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anonymize_discovered_paste() {
        let paste = DiscoveredPaste::new("test_source", "123", "test content")
            .with_author("John Doe")
            .with_url("https://example.com/paste/123");

        let config = AnonymizationConfig::default();
        let anonymized = anonymize_discovered_paste(paste, &config);

        // Author must be stripped
        assert!(anonymized.author.is_none());
        // URL must be stripped
        assert!(anonymized.url.is_empty());
    }

    #[test]
    fn test_verify_anonymity_passes() {
        assert!(verify_anonymity(Some("Normal title"), None));
    }

    #[test]
    fn test_verify_anonymity_fails_with_author() {
        assert!(!verify_anonymity(Some("title"), Some("John")));
    }

    #[test]
    fn test_verify_anonymity_fails_with_email_in_title() {
        assert!(!verify_anonymity(Some("Issue from user@example.com"), None));
    }

    #[test]
    fn test_anonymization_config_default() {
        let config = AnonymizationConfig::default();
        assert!(config.strip_authors);
        assert!(config.strip_urls);
        assert!(config.sanitize_titles);
    }
}
