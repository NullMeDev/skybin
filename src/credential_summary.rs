//! Credential Summary Extraction
//!
//! Extracts and summarizes critical credential information from content,
//! generating both a short title and a detailed header to prepend to pastes.

use once_cell::sync::Lazy;
use regex::Regex;

/// Patterns for extracting credentials
static EMAIL_PASS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@:]{4,})").unwrap()
});

static ULP_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(https?://[^\s]+)[\s\t|:]+([^\s@]+)[\s\t|:]+([^\s]{4,})").unwrap()
});

static GITHUB_PAT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(ghp_[a-zA-Z0-9]{36})").unwrap()
});

static GITHUB_OAUTH_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(gho_[a-zA-Z0-9]{36})").unwrap()
});

static OPENAI_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(sk-[a-zA-Z0-9]{48})").unwrap()
});

static AWS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(AKIA[0-9A-Z]{16})").unwrap()
});

static FIREBASE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(AIza[0-9A-Za-z_-]{35})").unwrap()
});

static SENDGRID_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43})").unwrap()
});

static SLACK_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(xox[baprs]-[0-9]{10,}-[a-zA-Z0-9-]+)").unwrap()
});

static DISCORD_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27})").unwrap()
});

static TG_BOT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([0-9]{8,10}:[A-Za-z0-9_-]{35})").unwrap()
});

static MONGODB_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(mongodb(?:\+srv)?://[^\s]+)").unwrap()
});

static POSTGRES_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(postgres(?:ql)?://[^\s]+)").unwrap()
});

static MYSQL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(mysql://[^\s]+)").unwrap()
});

static REDIS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(redis://[^\s]+)").unwrap()
});

/// Result of credential extraction
pub struct CredentialSummary {
    /// Short title like "2x API Key, 3x Email:Pass"
    pub title: String,
    /// Full formatted header to prepend
    pub header: String,
}

/// Extract and summarize credentials from content
/// Returns None if no credentials found
pub fn extract_credential_summary(content: &str, max_samples: usize) -> Option<CredentialSummary> {
    let mut summary_parts: Vec<String> = Vec::new();
    let mut title_parts: Vec<String> = Vec::new();

    // Email:password combos
    let email_passes: Vec<_> = EMAIL_PASS_PATTERN.captures_iter(content)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect();
    if !email_passes.is_empty() {
        let unique: Vec<_> = email_passes.iter()
            .take(max_samples)
            .collect();
        summary_parts.push(format!("EMAIL:PASS COMBOS ({} total, showing {}):", email_passes.len(), unique.len()));
        for ep in unique {
            summary_parts.push(format!("  - {}", ep));
        }
        title_parts.push(format!("{}x Email:Pass", email_passes.len()));
    }

    // URL:login:pass (stealer logs)
    let ulps: Vec<_> = ULP_PATTERN.captures_iter(content)
        .filter_map(|c| {
            let url = c.get(1)?.as_str();
            let login = c.get(2)?.as_str();
            let pass = c.get(3)?.as_str();
            Some((url.to_string(), login.to_string(), pass.to_string()))
        })
        .collect();
    if !ulps.is_empty() {
        summary_parts.push(format!("\nURL:LOGIN:PASS ({} total, showing {}):", ulps.len(), ulps.len().min(max_samples)));
        for (url, login, pwd) in ulps.iter().take(max_samples) {
            let display_url = if url.len() > 50 { format!("{}...", &url[..50]) } else { url.clone() };
            summary_parts.push(format!("  - {} | {} | {}", display_url, login, pwd));
        }
        title_parts.push(format!("{}x URL:Login:Pass", ulps.len()));
    }

    // API Keys
    let mut api_keys: Vec<(&str, String)> = Vec::new();
    
    for m in GITHUB_PAT_PATTERN.captures_iter(content).take(3) {
        if let Some(cap) = m.get(1) {
            api_keys.push(("GitHub PAT", cap.as_str().to_string()));
        }
    }
    for m in GITHUB_OAUTH_PATTERN.captures_iter(content).take(3) {
        if let Some(cap) = m.get(1) {
            api_keys.push(("GitHub OAuth", cap.as_str().to_string()));
        }
    }
    for m in OPENAI_PATTERN.captures_iter(content).take(3) {
        if let Some(cap) = m.get(1) {
            api_keys.push(("OpenAI", cap.as_str().to_string()));
        }
    }
    for m in AWS_PATTERN.captures_iter(content).take(3) {
        if let Some(cap) = m.get(1) {
            api_keys.push(("AWS Access Key", cap.as_str().to_string()));
        }
    }
    for m in FIREBASE_PATTERN.captures_iter(content).take(3) {
        if let Some(cap) = m.get(1) {
            api_keys.push(("Firebase/Google", cap.as_str().to_string()));
        }
    }
    for m in SENDGRID_PATTERN.captures_iter(content).take(3) {
        if let Some(cap) = m.get(1) {
            api_keys.push(("SendGrid", cap.as_str().to_string()));
        }
    }
    for m in SLACK_PATTERN.captures_iter(content).take(3) {
        if let Some(cap) = m.get(1) {
            api_keys.push(("Slack", cap.as_str().to_string()));
        }
    }

    if !api_keys.is_empty() {
        summary_parts.push(format!("\nAPI KEYS/TOKENS ({} total):", api_keys.len()));
        for (key_type, key) in api_keys.iter().take(max_samples) {
            let masked = if key.len() > 20 {
                format!("{}...{}", &key[..8], &key[key.len()-8..])
            } else {
                format!("{}...{}", &key[..4], &key[key.len()-4..])
            };
            summary_parts.push(format!("  - {}: {}", key_type, masked));
        }
        title_parts.push(format!("{}x API Key", api_keys.len()));
    }

    // Discord tokens
    let discord_tokens: Vec<_> = DISCORD_PATTERN.captures_iter(content)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect();
    if !discord_tokens.is_empty() {
        summary_parts.push(format!("\nDISCORD TOKENS ({} total):", discord_tokens.len()));
        for token in discord_tokens.iter().take(max_samples) {
            let masked = format!("{}...{}", &token[..10], &token[token.len()-10..]);
            summary_parts.push(format!("  - {}", masked));
        }
        title_parts.push(format!("{}x Discord Token", discord_tokens.len()));
    }

    // Telegram bot tokens
    let tg_tokens: Vec<_> = TG_BOT_PATTERN.captures_iter(content)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect();
    if !tg_tokens.is_empty() {
        summary_parts.push(format!("\nTELEGRAM BOT TOKENS ({} total):", tg_tokens.len()));
        for token in tg_tokens.iter().take(max_samples) {
            let masked = format!("{}...{}", &token[..8], &token[token.len()-8..]);
            summary_parts.push(format!("  - {}", masked));
        }
        title_parts.push(format!("{}x TG Bot Token", tg_tokens.len()));
    }

    // Database connections
    let mut db_conns: Vec<(&str, String)> = Vec::new();
    for m in MONGODB_PATTERN.captures_iter(content).take(2) {
        if let Some(cap) = m.get(1) {
            db_conns.push(("MongoDB", cap.as_str().to_string()));
        }
    }
    for m in POSTGRES_PATTERN.captures_iter(content).take(2) {
        if let Some(cap) = m.get(1) {
            db_conns.push(("PostgreSQL", cap.as_str().to_string()));
        }
    }
    for m in MYSQL_PATTERN.captures_iter(content).take(2) {
        if let Some(cap) = m.get(1) {
            db_conns.push(("MySQL", cap.as_str().to_string()));
        }
    }
    for m in REDIS_PATTERN.captures_iter(content).take(2) {
        if let Some(cap) = m.get(1) {
            db_conns.push(("Redis", cap.as_str().to_string()));
        }
    }

    if !db_conns.is_empty() {
        summary_parts.push(format!("\nDATABASE CONNECTIONS ({} total):", db_conns.len()));
        for (db_type, conn) in db_conns.iter().take(max_samples) {
            let display = if conn.len() > 60 {
                format!("{}...{}", &conn[..30], &conn[conn.len()-15..])
            } else {
                conn.clone()
            };
            summary_parts.push(format!("  - {}: {}", db_type, display));
        }
        title_parts.push(format!("{}x DB Conn", db_conns.len()));
    }

    // Private keys
    if content.contains("-----BEGIN") && content.contains("PRIVATE KEY-----") {
        let mut key_types = Vec::new();
        if content.contains("RSA PRIVATE KEY") { key_types.push("RSA"); }
        if content.contains("DSA PRIVATE KEY") { key_types.push("DSA"); }
        if content.contains("EC PRIVATE KEY") { key_types.push("EC"); }
        if content.contains("OPENSSH PRIVATE KEY") { key_types.push("OpenSSH"); }
        if content.contains("PGP PRIVATE KEY") { key_types.push("PGP"); }
        if key_types.is_empty() { key_types.push("Unknown"); }
        
        summary_parts.push(format!("\nPRIVATE KEYS: {}", key_types.join(", ")));
        title_parts.push(format!("{}x Private Key", key_types.len()));
    }

    if summary_parts.is_empty() {
        return None;
    }

    // Build title
    let title = title_parts.iter().take(4).cloned().collect::<Vec<_>>().join(", ");

    // Build header
    let mut header = String::new();
    header.push_str(&"=".repeat(60));
    header.push_str("\nCREDENTIAL SUMMARY\n");
    header.push_str(&"=".repeat(60));
    header.push('\n');
    header.push_str(&summary_parts.join("\n"));
    header.push('\n');
    header.push_str(&"=".repeat(60));
    header.push_str("\n\n");
    header.push_str(&" ".repeat(20));
    header.push_str("FULL CONTENT BELOW\n");
    header.push_str(&"-".repeat(60));
    header.push_str("\n\n");

    Some(CredentialSummary { title, header })
}

/// Prepend credential summary to content
/// Returns (new_title, new_content) tuple
pub fn prepend_summary(content: &str, fallback_title: &str) -> (String, String) {
    match extract_credential_summary(content, 10) {
        Some(summary) => {
            let new_content = format!("{}{}", summary.header, content);
            (summary.title, new_content)
        }
        None => (fallback_title.to_string(), content.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_pass_extraction() {
        let content = "test@example.com:password123\nuser@domain.org:secret456";
        let result = extract_credential_summary(content, 10);
        assert!(result.is_some());
        let summary = result.unwrap();
        assert!(summary.title.contains("Email:Pass"));
        assert!(summary.header.contains("EMAIL:PASS COMBOS"));
    }

    #[test]
    fn test_api_key_extraction() {
        let content = "my token is ghp_abcdefghijklmnopqrstuvwxyz123456 here";
        let result = extract_credential_summary(content, 10);
        assert!(result.is_some());
        let summary = result.unwrap();
        assert!(summary.title.contains("API Key"));
    }

    #[test]
    fn test_no_credentials() {
        let content = "This is just regular text with no credentials";
        let result = extract_credential_summary(content, 10);
        assert!(result.is_none());
    }

    #[test]
    fn test_prepend_summary() {
        let content = "test@example.com:password123";
        let (title, new_content) = prepend_summary(content, "Fallback Title");
        assert!(title.contains("Email:Pass"));
        assert!(new_content.contains("CREDENTIAL SUMMARY"));
        assert!(new_content.contains("test@example.com:password123"));
    }
}
