//! URL Classifier Module
//!
//! Classifies URLs based on host, path, and query parameters to identify
//! financial logins, identity auth, session tokens, and local admin panels.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

/// Classification result with score and tags
#[derive(Debug, Clone, Default)]
pub struct UrlClassification {
    pub score: i32,
    pub tags: Vec<String>,
    pub is_financial: bool,
    pub is_auth: bool,
    pub is_local: bool,
    pub redacted_url: String,
}

/// Financial/sensitive hosts (exact match or suffix match)
static FINANCIAL_HOSTS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // Google auth
        "accounts.google.com",
        "myaccount.google.com",
        // Microsoft
        "outlook.live.com",
        "login.microsoftonline.com",
        "login.live.com",
        // Gaming
        "us.battle.net",
        "eu.battle.net",
        // Banking (Latin America)
        "banesconline.com",
        "bod.bodmillenium.com",
        // E-wallets/Crypto
        "payeer.com",
        "neteller.com",
        "paypal.com",
        "skrill.com",
        "americankryptosbank.com",
        "coinbase.com",
        "binance.com",
        "kraken.com",
        // Payment processors
        "stripe.com",
        "square.com",
        "venmo.com",
        "cash.app",
        // Traditional banking
        "chase.com",
        "bankofamerica.com",
        "wellsfargo.com",
        "citi.com",
        "capitalone.com",
        "discover.com",
        "usbank.com",
        "pnc.com",
        // International banking
        "hsbc.com",
        "barclays.com",
        "santander.com",
        "ing.com",
        "deutsche-bank.de",
        // Hosting/Cloud
        "aws.amazon.com",
        "console.cloud.google.com",
        "portal.azure.com",
    ]
    .into_iter()
    .collect()
});

/// Auth-related path keywords
static AUTH_PATHS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "/login",
        "/signin",
        "/sign-in",
        "/auth",
        "/oauth",
        "/oauth2",
        "/account",
        "/myaccount",
        "/my-account",
        "/dashboard",
        "/settings",
        "/recovery",
        "/password_reset",
        "/password-reset",
        "/reset-password",
        "/forgot-password",
        "/2fa",
        "/mfa",
        "/verify",
        "/confirm",
        "/activate",
        "/session",
        "/sso",
        "/saml",
        "/callback",
    ]
});

/// Session/token query parameter names
static SESSION_PARAMS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "sid",
        "sidt",
        "SetSID",
        "osidt",
        "authuser",
        "token",
        "access_token",
        "refresh_token",
        "mcp_token",
        "session",
        "session_id",
        "sessionid",
        "redirect_uri",
        "state",
        "code",
        "nonce",
        "id_token",
        "auth",
        "apikey",
        "api_key",
    ]
});

/// Regex for long base64/signed blobs (potential tokens)
static LONG_TOKEN_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Za-z0-9_-]{40,}").unwrap());

/// Regex for LAN IPs
static LAN_IP_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:192\.168\.\d{1,3}\.\d{1,3}|10\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}|127\.0\.0\.1|localhost)").unwrap()
});

/// Classify a URL and return score + tags
pub fn classify_url(url: &str) -> UrlClassification {
    let mut result = UrlClassification {
        redacted_url: redact_url_values(url),
        ..Default::default()
    };

    let url_lower = url.to_lowercase();

    // Check for local/internal URLs
    if url_lower.starts_with("chrome-extension://")
        || url_lower.starts_with("file://")
        || url_lower.starts_with("about:")
        || LAN_IP_PATTERN.is_match(&url_lower)
    {
        result.is_local = true;
        result.tags.push("local_admin_panel".to_string());
        // Don't add score for local URLs
    }

    // Extract host from URL
    if let Some(host) = extract_host(&url_lower) {
        // Check financial hosts
        for fh in FINANCIAL_HOSTS.iter() {
            if host == *fh || host.ends_with(&format!(".{}", fh)) {
                result.is_financial = true;
                result.score += 5;
                result.tags.push("financial_login".to_string());
                break;
            }
        }
    }

    // Check auth paths
    for path in AUTH_PATHS.iter() {
        if url_lower.contains(path) {
            result.is_auth = true;
            result.score += 3;
            if !result.tags.contains(&"identity_auth".to_string()) {
                result.tags.push("identity_auth".to_string());
            }
            break;
        }
    }

    // Check session params in query string
    if let Some(query_start) = url_lower.find('?') {
        let query = &url_lower[query_start..];
        for param in SESSION_PARAMS.iter() {
            if query.contains(&format!("{}=", param)) || query.contains(&format!("{}&", param)) {
                result.score += 2;
                if !result.tags.contains(&"possible_session_token".to_string()) {
                    result.tags.push("possible_session_token".to_string());
                }
                break;
            }
        }

        // Check for long base64 blobs (potential tokens)
        if LONG_TOKEN_PATTERN.is_match(query) {
            result.score += 1;
            if !result.tags.contains(&"possible_session_token".to_string()) {
                result.tags.push("possible_session_token".to_string());
            }
        }
    }

    result
}

/// Classify multiple URLs in content and return aggregate results
pub fn classify_urls_in_content(content: &str) -> Vec<UrlClassification> {
    let url_pattern = Regex::new(r#"https?://[^\s\]<>"']+"#).unwrap();

    url_pattern
        .find_iter(content)
        .map(|m| classify_url(m.as_str()))
        .filter(|c| c.score > 0 || c.is_local)
        .collect()
}

/// Get aggregate tags from content
pub fn get_url_tags(content: &str) -> Vec<String> {
    let classifications = classify_urls_in_content(content);
    let mut tags: HashSet<String> = HashSet::new();

    for c in classifications {
        for tag in c.tags {
            tags.insert(tag);
        }
    }

    tags.into_iter().collect()
}

/// Extract host from URL
fn extract_host(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;

    let host_end = without_scheme.find('/').unwrap_or(without_scheme.len());
    let host_with_port = &without_scheme[..host_end];

    // Remove port if present
    let host = host_with_port
        .find(':')
        .map(|i| &host_with_port[..i])
        .unwrap_or(host_with_port);

    Some(host.to_string())
}

/// Redact sensitive values in URL query parameters
pub fn redact_url_values(url: &str) -> String {
    if let Some(query_start) = url.find('?') {
        let base = &url[..query_start];
        let query = &url[query_start + 1..];

        let redacted_params: Vec<String> = query
            .split('&')
            .map(|param| {
                if let Some(eq_pos) = param.find('=') {
                    let key = &param[..eq_pos];
                    let value = &param[eq_pos + 1..];

                    // Redact long values or known sensitive params
                    let is_sensitive = SESSION_PARAMS.iter().any(|p| key.to_lowercase() == *p);
                    let is_long = value.len() > 40;

                    if is_sensitive || is_long {
                        format!("{}=***len{}***", key, value.len())
                    } else {
                        param.to_string()
                    }
                } else {
                    param.to_string()
                }
            })
            .collect();

        format!("{}?{}", base, redacted_params.join("&"))
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_financial_host_detection() {
        let result = classify_url("https://accounts.google.com/signin?continue=...");
        assert!(result.is_financial);
        assert!(result.is_auth);
        assert!(result.tags.contains(&"financial_login".to_string()));
    }

    #[test]
    fn test_paypal_detection() {
        let result = classify_url("https://www.paypal.com/login?locale=en_US");
        assert!(result.is_financial);
        assert!(result.tags.contains(&"financial_login".to_string()));
    }

    #[test]
    fn test_auth_path_detection() {
        let result = classify_url("https://example.com/oauth2/callback?code=abc123");
        assert!(result.is_auth);
        assert!(result.tags.contains(&"identity_auth".to_string()));
    }

    #[test]
    fn test_session_token_detection() {
        let result = classify_url("https://example.com/api?access_token=xyz789&user=test");
        assert!(result.tags.contains(&"possible_session_token".to_string()));
    }

    #[test]
    fn test_lan_ip_detection() {
        let result = classify_url("http://192.168.1.1/admin/login");
        assert!(result.is_local);
        assert!(result.tags.contains(&"local_admin_panel".to_string()));
    }

    #[test]
    fn test_chrome_extension_detection() {
        let result = classify_url("chrome-extension://abcdef/popup.html");
        assert!(result.is_local);
        assert!(result.tags.contains(&"local_admin_panel".to_string()));
    }

    #[test]
    fn test_redact_sensitive_values() {
        let url =
            "https://example.com/callback?access_token=verylongtoken12345678901234567890&user=john";
        let redacted = redact_url_values(url);
        assert!(redacted.contains("access_token=***len"));
        assert!(redacted.contains("user=john")); // Short non-sensitive param kept
    }

    #[test]
    fn test_long_blob_detection() {
        let url = "https://example.com/auth?state=ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let result = classify_url(url);
        assert!(result.tags.contains(&"possible_session_token".to_string()));
    }
}
