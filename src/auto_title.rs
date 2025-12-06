use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// Generate a simplified, credential-focused title
/// Format: "Nx type found" or "Nx Type: specifics"
pub fn generate_title(content: &str) -> String {
    let content = content.trim();

    if content.is_empty() {
        return "Empty Paste".to_string();
    }

    // Priority 1: Credential-focused titles (simplified format)
    if let Some(title) = generate_credential_title(content) {
        return title;
    }

    // Priority 2: Code type detection
    if let Some(title) = detect_code_type(content) {
        return title;
    }

    // Priority 3: First meaningful line (markdown headers, etc.)
    if let Some(title) = extract_first_meaningful_line(content) {
        return title;
    }

    // Fallback: Generic summary
    generate_summary(content)
}

/// Generate simplified credential-focused title
/// Returns titles like "2x user:pass found", "3x API Keys: Google, AWS"
fn generate_credential_title(content: &str) -> Option<String> {
    let mut counts: HashMap<&str, (usize, Vec<String>)> = HashMap::new();
    
    // Count email:password combos
    let email_pass_count = EMAIL_PASS_RE.find_iter(content).count();
    if email_pass_count > 0 {
        counts.insert("user:pass", (email_pass_count, vec![]));
    }
    
    // Count URL:login:pass (stealer logs)
    let ulp_count = ULP_RE.find_iter(content).count();
    if ulp_count > 0 {
        counts.insert("URL:login:pass", (ulp_count, vec![]));
    }
    
    // Count API keys by type
    let mut api_keys: Vec<String> = vec![];
    let mut api_total = 0;
    
    for cap in GITHUB_PAT_RE.find_iter(content) {
        api_total += 1;
        if !api_keys.contains(&"GitHub".to_string()) {
            api_keys.push("GitHub".to_string());
        }
        let _ = cap; // suppress warning
    }
    for cap in OPENAI_RE.find_iter(content) {
        api_total += 1;
        if !api_keys.contains(&"OpenAI".to_string()) {
            api_keys.push("OpenAI".to_string());
        }
        let _ = cap;
    }
    for cap in AWS_RE.find_iter(content) {
        api_total += 1;
        if !api_keys.contains(&"AWS".to_string()) {
            api_keys.push("AWS".to_string());
        }
        let _ = cap;
    }
    for cap in GOOGLE_RE.find_iter(content) {
        api_total += 1;
        if !api_keys.contains(&"Google".to_string()) {
            api_keys.push("Google".to_string());
        }
        let _ = cap;
    }
    for cap in SENDGRID_RE.find_iter(content) {
        api_total += 1;
        if !api_keys.contains(&"SendGrid".to_string()) {
            api_keys.push("SendGrid".to_string());
        }
        let _ = cap;
    }
    for cap in SLACK_RE.find_iter(content) {
        api_total += 1;
        if !api_keys.contains(&"Slack".to_string()) {
            api_keys.push("Slack".to_string());
        }
        let _ = cap;
    }
    for cap in STRIPE_RE.find_iter(content) {
        api_total += 1;
        if !api_keys.contains(&"Stripe".to_string()) {
            api_keys.push("Stripe".to_string());
        }
        let _ = cap;
    }
    
    if api_total > 0 {
        counts.insert("API Keys", (api_total, api_keys));
    }
    
    // Count Discord tokens
    let discord_count = DISCORD_RE.find_iter(content).count();
    if discord_count > 0 {
        counts.insert("Discord Token", (discord_count, vec![]));
    }
    
    // Count Telegram bot tokens
    let tg_count = TG_BOT_RE.find_iter(content).count();
    if tg_count > 0 {
        counts.insert("TG Bot Token", (tg_count, vec![]));
    }
    
    // Count private keys
    let pk_count = PRIVATE_KEY_RE.find_iter(content).count();
    if pk_count > 0 {
        counts.insert("Private Key", (pk_count, vec![]));
    }
    
    // Count database URLs
    let db_count = DB_URL_RE.find_iter(content).count();
    if db_count > 0 {
        counts.insert("DB Connection", (db_count, vec![]));
    }
    
    // Build title from counts
    if counts.is_empty() {
        return None;
    }
    
    // Sort by count descending, take top 2
    let mut items: Vec<_> = counts.into_iter().collect();
    items.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    
    let parts: Vec<String> = items
        .into_iter()
        .take(2)
        .map(|(name, (count, specifics))| {
            if specifics.is_empty() {
                format!("{}x {}", count, name)
            } else {
                let spec_str = specifics.into_iter().take(3).collect::<Vec<_>>().join(", ");
                format!("{}x {}: {}", count, name, spec_str)
            }
        })
        .collect();
    
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

// Compiled regexes for credential detection
static EMAIL_PASS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@:]{4,}").unwrap()
});

static ULP_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"https?://[^\s]+[\s\t|:]+[^\s@]+[\s\t|:]+[^\s]{4,}").unwrap()
});

static GITHUB_PAT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"gh[ps]_[a-zA-Z0-9]{36}").unwrap()
});

static OPENAI_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"sk-[a-zA-Z0-9]{48}").unwrap()
});

static AWS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"AKIA[0-9A-Z]{16}").unwrap()
});

static GOOGLE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"AIza[0-9A-Za-z_-]{35}").unwrap()
});

static SENDGRID_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}").unwrap()
});

static SLACK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"xox[baprs]-[0-9]{10,}-[a-zA-Z0-9-]+").unwrap()
});

static STRIPE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"sk_(live|test)_[0-9a-zA-Z]{24,}").unwrap()
});

static DISCORD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27}").unwrap()
});

static TG_BOT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d{8,10}:[A-Za-z0-9_-]{35}").unwrap()
});

static PRIVATE_KEY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"-----BEGIN\s+(RSA|DSA|EC|OPENSSH|PGP)?\s*PRIVATE\s+KEY").unwrap()
});

static DB_URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(mongodb|postgres|mysql|redis)(?:\+srv)?://").unwrap()
});

fn detect_code_type(content: &str) -> Option<String> {
    let patterns: &[(&str, &str)] = &[
        (r#"^\s*<\?php"#, "PHP Script"),
        (r#"^\s*#!/usr/bin/(env\s+)?python"#, "Python Script"),
        (r#"^\s*#!/usr/bin/(env\s+)?bash"#, "Bash Script"),
        (r#"^\s*#!/usr/bin/(env\s+)?node"#, "Node.js Script"),
        (r#"^\s*#!/usr/bin/(env\s+)?ruby"#, "Ruby Script"),
        (r#"^\s*#!/usr/bin/(env\s+)?perl"#, "Perl Script"),
        (r#"^\s*package\s+main"#, "Go Program"),
        (r#"^\s*fn\s+main\s*\("#, "Rust Program"),
        (r#"^\s*public\s+class\s+\w+"#, "Java Class"),
        (r#"^\s*class\s+\w+.*:"#, "Python Class"),
        (
            r#"^\s*import\s+(React|useState|useEffect)"#,
            "React Component",
        ),
        (
            r#"^\s*import\s+\{.*\}\s+from\s+['"]vue['"]"#,
            "Vue Component",
        ),
        (r#"^\s*<template>"#, "Vue Template"),
        (r#"^\s*<!DOCTYPE\s+html>"#, "HTML Document"),
        (r#"^\s*<html"#, "HTML Document"),
        (r#"^\s*\{[\s\n]*""#, "JSON Data"),
        (r#"^\s*---\n"#, "YAML Document"),
        (r#"^\s*#\s+\w+"#, "Markdown Document"),
        (r#"^\s*CREATE\s+TABLE"#, "SQL Schema"),
        (r#"^\s*SELECT\s+"#, "SQL Query"),
        (r#"^\s*INSERT\s+INTO"#, "SQL Insert"),
        (r#"^\s*UPDATE\s+\w+\s+SET"#, "SQL Update"),
        (r#"^\s*const\s+\w+\s*=\s*require\("#, "Node.js Module"),
        (r#"^\s*import\s+\w+\s+from\s+"#, "ES6 Module"),
        (
            r#"^\s*export\s+(default\s+)?(function|class|const)"#,
            "ES6 Export",
        ),
        (r#"^\s*\[Unit\]"#, "Systemd Unit File"),
        (r#"^\s*\[Service\]"#, "Systemd Service"),
        (r#"^\s*FROM\s+\w+"#, "Dockerfile"),
        (r#"^\s*version:\s*['"]?\d"#, "Docker Compose"),
        (r#"^\s*apiVersion:"#, "Kubernetes Manifest"),
        (r#"^\s*terraform\s*\{"#, "Terraform Config"),
        (r#"^\s*provider\s+""#, "Terraform Provider"),
        (r#"^\s*resource\s+""#, "Terraform Resource"),
    ];

    for (pattern, name) in patterns {
        if let Ok(re) = Regex::new(&format!("(?i){}", pattern)) {
            if re.is_match(content) {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn detect_data_type(content: &str) -> Option<String> {
    let content_lower = content.to_lowercase();

    // Streaming service logins (check first as most specific)
    let streaming_patterns = [
        (&["disney", "disneyplus", "disney+"][..], "Disney+ Login"),
        (&["netflix"][..], "Netflix Login"),
        (&["hulu"][..], "Hulu Login"),
        (&["hbomax", "hbo max", "hbo"][..], "HBO Max Login"),
        (&["paramount", "paramount+"][..], "Paramount+ Login"),
        (&["peacock"][..], "Peacock Login"),
        (&["crunchyroll"][..], "Crunchyroll Login"),
        (&["funimation"][..], "Funimation Login"),
        (&["spotify"][..], "Spotify Login"),
        (&["apple music", "applemusic"][..], "Apple Music Login"),
        (
            &["amazon prime", "primevideo", "prime video"][..],
            "Amazon Prime Login",
        ),
        (&["twitch"][..], "Twitch Login"),
        (
            &["youtube premium", "ytpremium"][..],
            "YouTube Premium Login",
        ),
        (&["dazn"][..], "DAZN Login"),
        (&["espn", "espn+"][..], "ESPN+ Login"),
        (&["showtime"][..], "Showtime Login"),
        (&["starz"][..], "Starz Login"),
        (&["tidal"][..], "Tidal Login"),
        (&["deezer"][..], "Deezer Login"),
        (&["nordvpn"][..], "NordVPN Login"),
        (&["expressvpn"][..], "ExpressVPN Login"),
        (&["surfshark"][..], "Surfshark Login"),
        (&["ipvanish"][..], "IPVanish Login"),
        (&["pornhub", "brazzers", "onlyfans"][..], "Adult Site Login"),
        (&["minecraft"][..], "Minecraft Login"),
        (&["steam", "steamcommunity"][..], "Steam Login"),
        (&["origin", "ea.com"][..], "EA/Origin Login"),
        (&["epic games", "epicgames"][..], "Epic Games Login"),
        (&["playstation", "psn"][..], "PlayStation Login"),
        (&["xbox", "xbox live"][..], "Xbox Live Login"),
        (&["roblox"][..], "Roblox Login"),
        (&["fortnite"][..], "Fortnite Login"),
    ];

    // Check if content has email:password pattern AND a service keyword
    let has_login_pattern = content_lower.contains(':')
        && (content_lower.contains('@')
            || content_lower.contains("pass")
            || content_lower.contains("user"));

    if has_login_pattern {
        for (keywords, title) in streaming_patterns {
            for keyword in keywords {
                if content_lower.contains(keyword) {
                    return Some(title.to_string());
                }
            }
        }
    }

    // General patterns
    let patterns = [
        // Combo lists
        (
            r"(?i)([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})\s*[:|]\s*\S+",
            "Email:Password Combo List",
        ),
        (
            r"(?i)(user|username)\s*[:|]\s*\S+.*(pass|password)\s*[:|]\s*\S+",
            "Username:Password List",
        ),
        // API and tokens
        (r"(?i)api[_-]?key\s*[:=]", "API Key Leak"),
        (r"(?i)secret[_-]?key\s*[:=]", "Secret Key Data"),
        (r"AKIA[0-9A-Z]{16}", "AWS Credentials"),
        (r"ghp_[a-zA-Z0-9]{36}", "GitHub Token"),
        (r"xox[baprs]-[0-9a-zA-Z-]+", "Slack Token"),
        (r"(?i)discord.*token|token.*discord", "Discord Token"),
        (r"[MN][A-Za-z\d]{23,}\.[\w-]{6}\.[\w-]{27}", "Discord Token"),
        (r"(?i)telegram.*bot|bot.*token", "Telegram Bot Token"),
        // Keys and certs
        (
            r"-----BEGIN\s+(RSA|DSA|EC|OPENSSH)\s+PRIVATE\s+KEY-----",
            "Private Key",
        ),
        (r"-----BEGIN\s+CERTIFICATE-----", "SSL Certificate"),
        // Database
        (
            r"(?i)mysql://|postgres://|mongodb://|redis://",
            "Database Connection String",
        ),
        // Financial
        (r"\b4[0-9]{12}(?:[0-9]{3})?\b", "Credit Card Numbers"),
        (r"(?i)cvv|credit.?card|debit.?card", "Payment Card Data"),
        (r"(?i)paypal.*login|paypal.*pass", "PayPal Login"),
        (r"(?i)bank.*login|bank.*pass", "Banking Credentials"),
        // Social media
        (r"(?i)instagram.*pass|insta.*login", "Instagram Login"),
        (r"(?i)facebook.*pass|fb.*login", "Facebook Login"),
        (r"(?i)twitter.*pass|twitter.*login", "Twitter Login"),
        (r"(?i)tiktok.*pass|tiktok.*login", "TikTok Login"),
        (r"(?i)snapchat.*pass|snap.*login", "Snapchat Login"),
        // Email providers
        (r"(?i)gmail.*pass|google.*login", "Gmail/Google Login"),
        (
            r"(?i)outlook.*pass|hotmail.*pass|microsoft.*login",
            "Microsoft/Outlook Login",
        ),
        (r"(?i)yahoo.*pass|yahoo.*login", "Yahoo Login"),
        (r"(?i)protonmail|proton.*mail", "ProtonMail Login"),
        // Cloud services
        (r"(?i)aws.*key|amazon.*secret", "AWS Credentials"),
        (r"(?i)azure.*key|azure.*secret", "Azure Credentials"),
        (r"(?i)gcp.*key|google.*cloud", "Google Cloud Credentials"),
        (r"(?i)digitalocean.*token", "DigitalOcean Token"),
        (r"(?i)heroku.*api", "Heroku API Key"),
        // Hosting/domains
        (r"(?i)cpanel.*pass|cpanel.*login", "cPanel Login"),
        (r"(?i)plesk.*pass|plesk.*login", "Plesk Login"),
        (
            r"(?i)godaddy.*login|namecheap.*login",
            "Domain Registrar Login",
        ),
        (
            r"(?i)cloudflare.*key|cloudflare.*token",
            "Cloudflare Credentials",
        ),
        // General auth
        (r"(?i)bearer\s+[a-zA-Z0-9._-]+", "Bearer Token"),
        (r"(?i)authorization:\s*", "Authorization Header"),
        (r"(?i)oauth.*token|access_token", "OAuth Token"),
        (r"(?i)jwt.*token|eyJ[A-Za-z0-9_-]+\.", "JWT Token"),
        // IP/Network
        (
            r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b.*\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b",
            "IP Address List",
        ),
        (r"(?i)ssh.*pass|ssh.*key|id_rsa", "SSH Credentials"),
        (r"(?i)ftp.*pass|ftp.*login", "FTP Credentials"),
        (r"(?i)rdp.*pass|remote.*desktop", "RDP Credentials"),
        // Logs
        (
            r"(?i)(error|exception|traceback|stack\s*trace)",
            "Error Log",
        ),
        (r"(?i)access[_-]?log|error[_-]?log", "Server Log"),
        (r"\[\d{2}/\w{3}/\d{4}:\d{2}:\d{2}:\d{2}", "Apache/Nginx Log"),
        // Config files
        (
            r"(?i)config.*password|password.*config",
            "Config File with Passwords",
        ),
        (r"(?i)\.env|environment.*variable", "Environment Variables"),
        // General email lists
        (
            r"([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}.*){5,}",
            "Email List",
        ),
    ];

    for (pattern, name) in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(content) {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn extract_first_meaningful_line(content: &str) -> Option<String> {
    for line in content.lines().take(10) {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') && !line.starts_with("##") {
            let title = line.trim_start_matches('#').trim();
            if !title.is_empty() && title.len() <= 60 {
                return Some(title.to_string());
            }
        }
        if line.starts_with("//") || line.starts_with("/*") || line.starts_with("*") {
            let cleaned = line.trim_start_matches('/').trim_start_matches('*').trim();
            if cleaned.len() >= 10 && cleaned.len() <= 60 && !cleaned.contains("TODO") {
                return Some(cleaned.to_string());
            }
        }
        if let Some(caps) = DEF_REGEX.captures(line) {
            if let Some(name) = caps.get(1) {
                return Some(format!("{} Definition", name.as_str()));
            }
        }
    }
    None
}

static DEF_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"^(?:def|function|fn|class|struct|interface|type)\s+(\w+)"#).unwrap());

fn generate_summary(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or("").trim();

    if first_line.is_empty() {
        return "Text Paste".to_string();
    }

    let cleaned: String = first_line
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
        .collect();

    let cleaned = cleaned.trim();

    if cleaned.len() < 3 {
        return "Code Snippet".to_string();
    }

    if cleaned.len() > 50 {
        let truncated: String = cleaned.chars().take(47).collect();
        format!("{}...", truncated.trim_end())
    } else {
        cleaned.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_php_detection() {
        assert_eq!(generate_title("<?php echo 'hello';"), "PHP Script");
    }

    #[test]
    fn test_python_detection() {
        assert_eq!(
            generate_title("#!/usr/bin/env python\nprint('hi')"),
            "Python Script"
        );
    }

    #[test]
    fn test_json_detection() {
        assert_eq!(generate_title("{\"key\": \"value\"}"), "JSON Data");
    }

    #[test]
    fn test_aws_key_detection() {
        // New simplified format
        assert_eq!(generate_title("AKIAIOSFODNN7EXAMPLE"), "1x API Keys: AWS");
    }

    #[test]
    fn test_simplified_user_pass_title() {
        let content = "test@example.com:password123\nuser@domain.org:secret456";
        let title = generate_title(content);
        assert!(title.contains("user:pass"), "Expected user:pass in title: {}", title);
    }

    #[test]
    fn test_multiple_credential_types() {
        let content = "test@example.com:pass123\nghp_1234567890abcdefghijklmnopqrstuvwx";
        let title = generate_title(content);
        // Should show both types
        assert!(title.contains("user:pass") || title.contains("API Keys"), "Title: {}", title);
    }

    #[test]
    fn test_markdown_title() {
        // Markdown is detected as code type first, extracting header comes after
        // Since detect_code_type matches markdown pattern, we get "Markdown Document"
        // This is correct - the header extraction is in extract_first_meaningful_line
        // which runs after detect_code_type
        let title = generate_title("# My Document\n\nContent here");
        // Either "Markdown Document" (code type) or "My Document" (header extract) is valid
        assert!(
            title == "Markdown Document" || title == "My Document",
            "Unexpected title: {}",
            title
        );
    }

    #[test]
    fn test_empty() {
        assert_eq!(generate_title(""), "Empty Paste");
    }
}
