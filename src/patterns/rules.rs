use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

/// Pattern severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low = 0,
    Moderate = 1,
    High = 2,
    Critical = 3,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Low => write!(f, "low"),
            Severity::Moderate => write!(f, "moderate"),
            Severity::High => write!(f, "high"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

/// A pattern rule for detecting sensitive data
#[derive(Debug, Clone)]
pub struct PatternRule {
    pub name: String,
    pub regex: Regex,
    pub severity: Severity,
    pub category: String,
}

impl PatternRule {
    pub fn new(
        name: impl Into<String>,
        pattern: &str,
        severity: Severity,
        category: impl Into<String>,
    ) -> Result<Self, regex::Error> {
        Ok(PatternRule {
            name: name.into(),
            regex: Regex::new(pattern)?,
            severity,
            category: category.into(),
        })
    }
}

lazy_static! {
    /// Built-in pattern rules for sensitive data detection
    pub static ref BUILTIN_PATTERNS: HashMap<String, PatternRule> = {
        let mut m = HashMap::new();

        // AWS API Keys
        m.insert(
            "aws_key".to_string(),
            PatternRule::new(
                "AWS Access Key",
                r"(?i)AKIA[0-9A-Z]{16}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // GitHub Personal Access Tokens
        m.insert(
            "github_token".to_string(),
            PatternRule::new(
                "GitHub Token",
                r"gh[pousr]{2}_[A-Za-z0-9_]{36,255}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Stripe API Keys
        m.insert(
            "stripe_key".to_string(),
            PatternRule::new(
                "Stripe API Key",
                r"sk_(?:live|test)_[0-9a-zA-Z]{20,32}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Generic API Keys (common patterns)
        m.insert(
            "generic_api_key".to_string(),
            PatternRule::new(
                "Generic API Key",
                r"(?i)api[_-]?key\s*[:=]\s*[a-zA-Z0-9]{20,}",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // Private SSH Keys
        m.insert(
            "ssh_private_key".to_string(),
            PatternRule::new(
                "SSH Private Key",
                r"-----BEGIN RSA PRIVATE KEY-----",
                Severity::Critical,
                "keys",
            ).unwrap(),
        );

        // PGP Private Keys
        m.insert(
            "pgp_private_key".to_string(),
            PatternRule::new(
                "PGP Private Key",
                r"-----BEGIN PGP PRIVATE KEY BLOCK-----",
                Severity::Critical,
                "keys",
            ).unwrap(),
        );

        // OpenSSH Private Keys
        m.insert(
            "openssh_private_key".to_string(),
            PatternRule::new(
                "OpenSSH Private Key",
                r"-----BEGIN OPENSSH PRIVATE KEY-----",
                Severity::Critical,
                "keys",
            ).unwrap(),
        );

        // Credit Cards - Match major card prefixes with Luhn-like structure
        // Visa: 4xxx, MC: 51-55, Amex: 34/37, Discover: 6011/65
        // Supports optional dashes or spaces between digit groups
        m.insert(
            "credit_card".to_string(),
            PatternRule::new(
                "Credit Card Number",
                r"\b(?:4[0-9]{3}[- ]?[0-9]{4}[- ]?[0-9]{4}[- ]?[0-9]{4}|5[1-5][0-9]{2}[- ]?[0-9]{4}[- ]?[0-9]{4}[- ]?[0-9]{4}|3[47][0-9]{2}[- ]?[0-9]{6}[- ]?[0-9]{5}|6(?:011|5[0-9]{2})[- ]?[0-9]{4}[- ]?[0-9]{4}[- ]?[0-9]{4})\b",
                Severity::Critical,
                "financial",
            ).unwrap(),
        );

        // Database Connection Strings
        m.insert(
            "db_connection".to_string(),
            PatternRule::new(
                "Database Connection String",
                r"(?i)(?:mysql|postgres|mssql|mongodb)://[^\s]+",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // Email + Password combos
        m.insert(
            "email_password".to_string(),
            PatternRule::new(
                "Email:Password Combo",
                r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}[:=\s]+\w+",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // AWS Account ID - require context to reduce false positives
        m.insert(
            "aws_account_id".to_string(),
            PatternRule::new(
                "AWS Account ID",
                r"(?i)(?:aws|account[_-]?id|arn:aws)\s*[:=]?\s*\d{12}\b",
                Severity::Moderate,
                "identifiers",
            ).unwrap(),
        );

        // Private IPv4 CIDR ranges
        m.insert(
            "private_ip_cidr".to_string(),
            PatternRule::new(
                "Private IP/CIDR",
                r"(?:10|172\.(?:1[6-9]|2\d|3[01])|192\.168)(?:\.\d{1,3}){2}(?:/\d+)?",
                Severity::Moderate,
                "network",
            ).unwrap(),
        );

        // Slack Webhook URLs
        m.insert(
            "slack_webhook".to_string(),
            PatternRule::new(
                "Slack Webhook URL",
                r"https://hooks\.slack\.com/services/T[A-Z0-9]{8}/B[A-Z0-9]{8}/[A-Za-z0-9]{24}",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // Mailchimp API Keys
        m.insert(
            "mailchimp_key".to_string(),
            PatternRule::new(
                "Mailchimp API Key",
                r"[0-9a-f]{32}-us\d{1,2}",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // Discord Tokens
        m.insert(
            "discord_token".to_string(),
            PatternRule::new(
                "Discord Token",
                r"[MN][A-Za-z\d]{23,}\.[\w-]{6}\.[\w-]{27}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Discord Webhook
        m.insert(
            "discord_webhook".to_string(),
            PatternRule::new(
                "Discord Webhook",
                r"https://(?:ptb\.|canary\.)?discord(?:app)?\.com/api/webhooks/\d+/[\w-]+",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // Telegram Bot Token
        m.insert(
            "telegram_token".to_string(),
            PatternRule::new(
                "Telegram Bot Token",
                r"\d{8,10}:[A-Za-z0-9_-]{35}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Twitch OAuth Token
        m.insert(
            "twitch_token".to_string(),
            PatternRule::new(
                "Twitch OAuth Token",
                r"(?i)oauth:[a-z0-9]{30}",
                Severity::Critical,
                "streaming",
            ).unwrap(),
        );

        // Spotify Client Secret
        m.insert(
            "spotify_secret".to_string(),
            PatternRule::new(
                "Spotify Client Secret",
                r"(?i)spotify[_-]?(?:client[_-]?)?secret\s*[:=]\s*[a-f0-9]{32}",
                Severity::High,
                "streaming",
            ).unwrap(),
        );

        // Netflix Cookies/Tokens
        m.insert(
            "netflix_cookie".to_string(),
            PatternRule::new(
                "Netflix Session",
                r"(?i)NetflixId=[A-Za-z0-9%_-]{50,}",
                Severity::Critical,
                "streaming",
            ).unwrap(),
        );

        // YouTube API Key
        m.insert(
            "youtube_key".to_string(),
            PatternRule::new(
                "YouTube API Key",
                r"AIza[0-9A-Za-z\-_]{35}",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // Google OAuth Token
        m.insert(
            "google_oauth".to_string(),
            PatternRule::new(
                "Google OAuth Token",
                r"ya29\.[0-9A-Za-z\-_]+",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Facebook Access Token
        m.insert(
            "facebook_token".to_string(),
            PatternRule::new(
                "Facebook Access Token",
                r"EAA[A-Za-z0-9]{100,}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Twitter Bearer Token
        m.insert(
            "twitter_bearer".to_string(),
            PatternRule::new(
                "Twitter Bearer Token",
                r"AAAAAAAAAAAAAAAAAAAAAA[A-Za-z0-9%]{40,}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Heroku API Key
        m.insert(
            "heroku_key".to_string(),
            PatternRule::new(
                "Heroku API Key",
                r"(?i)heroku[_-]?api[_-]?key\s*[:=]\s*[a-f0-9-]{36}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Sendgrid API Key
        m.insert(
            "sendgrid_key".to_string(),
            PatternRule::new(
                "Sendgrid API Key",
                r"SG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Square Access Token
        m.insert(
            "square_token".to_string(),
            PatternRule::new(
                "Square Access Token",
                r"sq0atp-[0-9A-Za-z\-_]{22}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // PayPal Client Secret
        m.insert(
            "paypal_secret".to_string(),
            PatternRule::new(
                "PayPal Client Secret",
                r"(?i)paypal[_-]?(?:client[_-]?)?secret\s*[:=]\s*[A-Za-z0-9_-]{40,}",
                Severity::Critical,
                "financial",
            ).unwrap(),
        );

        // DigitalOcean Token
        m.insert(
            "digitalocean_token".to_string(),
            PatternRule::new(
                "DigitalOcean Token",
                r"dop_v1_[a-f0-9]{64}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Azure Storage Key
        m.insert(
            "azure_storage".to_string(),
            PatternRule::new(
                "Azure Storage Key",
                r"(?i)AccountKey=[A-Za-z0-9+/=]{88}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // JWT Token
        m.insert(
            "jwt_token".to_string(),
            PatternRule::new(
                "JWT Token",
                r"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // Generic Password in Config
        m.insert(
            "password_config".to_string(),
            PatternRule::new(
                "Password in Config",
                r#"(?i)(?:password|passwd|pwd)\s*[:=]\s*['"]?[^\s'"]{8,}['"]?"#,
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // Bearer Token
        m.insert(
            "bearer_token".to_string(),
            PatternRule::new(
                "Bearer Token",
                r"(?i)bearer\s+[A-Za-z0-9\-_\.]{20,}",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // NPM Token
        m.insert(
            "npm_token".to_string(),
            PatternRule::new(
                "NPM Token",
                r"npm_[A-Za-z0-9]{36}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Docker Registry Auth
        m.insert(
            "docker_auth".to_string(),
            PatternRule::new(
                "Docker Registry Auth",
                r"(?i)docker[_-]?(?:auth|password|token)\s*[:=]\s*[A-Za-z0-9+/=]{20,}",
                Severity::Critical,
                "credentials",
            ).unwrap(),
        );

        // Crunchyroll Credentials
        m.insert(
            "crunchyroll_creds".to_string(),
            PatternRule::new(
                "Crunchyroll Credentials",
                r"(?i)crunchyroll[_-]?(?:user|pass|email|token)\s*[:=]\s*[^\s]+",
                Severity::High,
                "streaming",
            ).unwrap(),
        );

        // Hulu Token
        m.insert(
            "hulu_token".to_string(),
            PatternRule::new(
                "Hulu Session",
                r"(?i)hulu[_-]?(?:session|token|auth)\s*[:=]\s*[A-Za-z0-9_-]{30,}",
                Severity::Critical,
                "streaming",
            ).unwrap(),
        );

        // Disney+ Token
        m.insert(
            "disney_token".to_string(),
            PatternRule::new(
                "Disney+ Session",
                r"(?i)disney[_-]?(?:plus|session|token|auth)\s*[:=]\s*[A-Za-z0-9_-]{30,}",
                Severity::Critical,
                "streaming",
            ).unwrap(),
        );

        // HBO Max Token
        m.insert(
            "hbo_token".to_string(),
            PatternRule::new(
                "HBO Max Session",
                r"(?i)hbo[_-]?(?:max)?[_-]?(?:session|token|auth)\s*[:=]\s*[A-Za-z0-9_-]{30,}",
                Severity::Critical,
                "streaming",
            ).unwrap(),
        );

        // Steam API Key
        m.insert(
            "steam_key".to_string(),
            PatternRule::new(
                "Steam API Key",
                r"[0-9A-F]{32}",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // VPN Config Credentials
        m.insert(
            "vpn_creds".to_string(),
            PatternRule::new(
                "VPN Credentials",
                r"(?i)(?:openvpn|wireguard|vpn)[_-]?(?:user|pass|key|auth)\s*[:=]\s*[^\s]+",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // Cloudflare API Token
        m.insert(
            "cloudflare_token".to_string(),
            PatternRule::new(
                "Cloudflare API Token",
                r"[A-Za-z0-9_-]{40}",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        // Generic Secret Key
        m.insert(
            "secret_key".to_string(),
            PatternRule::new(
                "Secret Key",
                r"(?i)secret[_-]?key\s*[:=]\s*[A-Za-z0-9_-]{20,}",
                Severity::High,
                "credentials",
            ).unwrap(),
        );

        m
    };
}

/// Get all enabled patterns based on configuration
pub fn get_enabled_patterns(
    config: &crate::config::PatternsConfig,
    custom_patterns: &[crate::config::CustomPattern],
) -> Vec<PatternRule> {
    let mut patterns = Vec::new();

    if config.aws_keys {
        patterns.push(BUILTIN_PATTERNS["aws_key"].clone());
        patterns.push(BUILTIN_PATTERNS["aws_account_id"].clone());
    }
    if config.generic_api_keys {
        patterns.push(BUILTIN_PATTERNS["generic_api_key"].clone());
        patterns.push(BUILTIN_PATTERNS["stripe_key"].clone());
        patterns.push(BUILTIN_PATTERNS["github_token"].clone());
        patterns.push(BUILTIN_PATTERNS["mailchimp_key"].clone());
        patterns.push(BUILTIN_PATTERNS["slack_webhook"].clone());
    }
    if config.private_keys {
        patterns.push(BUILTIN_PATTERNS["ssh_private_key"].clone());
        patterns.push(BUILTIN_PATTERNS["pgp_private_key"].clone());
        patterns.push(BUILTIN_PATTERNS["openssh_private_key"].clone());
    }
    if config.credit_cards {
        patterns.push(BUILTIN_PATTERNS["credit_card"].clone());
    }
    if config.db_credentials {
        patterns.push(BUILTIN_PATTERNS["db_connection"].clone());
    }
    if config.email_password_combos {
        patterns.push(BUILTIN_PATTERNS["email_password"].clone());
    }
    if config.ip_cidr {
        patterns.push(BUILTIN_PATTERNS["private_ip_cidr"].clone());
    }
    if config.discord_tokens {
        patterns.push(BUILTIN_PATTERNS["discord_token"].clone());
        patterns.push(BUILTIN_PATTERNS["discord_webhook"].clone());
        patterns.push(BUILTIN_PATTERNS["telegram_token"].clone());
    }
    if config.oauth_tokens {
        patterns.push(BUILTIN_PATTERNS["google_oauth"].clone());
        patterns.push(BUILTIN_PATTERNS["facebook_token"].clone());
        patterns.push(BUILTIN_PATTERNS["twitter_bearer"].clone());
        patterns.push(BUILTIN_PATTERNS["bearer_token"].clone());
    }
    if config.streaming_creds {
        patterns.push(BUILTIN_PATTERNS["twitch_token"].clone());
        patterns.push(BUILTIN_PATTERNS["spotify_secret"].clone());
        patterns.push(BUILTIN_PATTERNS["netflix_cookie"].clone());
        patterns.push(BUILTIN_PATTERNS["crunchyroll_creds"].clone());
        patterns.push(BUILTIN_PATTERNS["hulu_token"].clone());
        patterns.push(BUILTIN_PATTERNS["disney_token"].clone());
        patterns.push(BUILTIN_PATTERNS["hbo_token"].clone());
    }
    if config.jwt_tokens {
        patterns.push(BUILTIN_PATTERNS["jwt_token"].clone());
    }
    if config.payment_keys {
        patterns.push(BUILTIN_PATTERNS["square_token"].clone());
        patterns.push(BUILTIN_PATTERNS["paypal_secret"].clone());
    }
    if config.cloud_tokens {
        patterns.push(BUILTIN_PATTERNS["heroku_key"].clone());
        patterns.push(BUILTIN_PATTERNS["sendgrid_key"].clone());
        patterns.push(BUILTIN_PATTERNS["digitalocean_token"].clone());
        patterns.push(BUILTIN_PATTERNS["azure_storage"].clone());
        patterns.push(BUILTIN_PATTERNS["npm_token"].clone());
        patterns.push(BUILTIN_PATTERNS["docker_auth"].clone());
        patterns.push(BUILTIN_PATTERNS["youtube_key"].clone());
        patterns.push(BUILTIN_PATTERNS["vpn_creds"].clone());
        patterns.push(BUILTIN_PATTERNS["password_config"].clone());
        patterns.push(BUILTIN_PATTERNS["secret_key"].clone());
    }

    // Add custom patterns
    for custom in custom_patterns {
        if let Ok(rule) = PatternRule::new(
            &custom.name,
            &custom.regex,
            parse_severity(&custom.severity),
            "custom",
        ) {
            patterns.push(rule);
        }
    }

    patterns
}

/// Parse severity string to Severity enum
pub fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "moderate" => Severity::Moderate,
        _ => Severity::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_patterns_loaded() {
        assert!(!BUILTIN_PATTERNS.is_empty());
        assert!(BUILTIN_PATTERNS.contains_key("aws_key"));
        assert!(BUILTIN_PATTERNS.contains_key("ssh_private_key"));
        assert!(BUILTIN_PATTERNS.contains_key("credit_card"));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Low < Severity::Moderate);
        assert!(Severity::Moderate < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }

    #[test]
    fn test_aws_key_detection() {
        let pattern = &BUILTIN_PATTERNS["aws_key"];
        assert!(pattern.regex.is_match("AKIAIOSFODNN7EXAMPLE"));
        assert!(!pattern.regex.is_match("INVALIDKEY123456789"));
    }

    #[test]
    fn test_private_key_detection() {
        let pattern = &BUILTIN_PATTERNS["ssh_private_key"];
        assert!(pattern.regex.is_match("-----BEGIN RSA PRIVATE KEY-----"));
    }

    #[test]
    fn test_credit_card_pattern() {
        let pattern = &BUILTIN_PATTERNS["credit_card"];
        assert!(pattern.regex.is_match("4532015112830366"));
        assert!(pattern.regex.is_match("4532-0151-1283-0366"));
        assert!(pattern.regex.is_match("4532 0151 1283 0366"));
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Low.to_string(), "low");
        assert_eq!(Severity::Moderate.to_string(), "moderate");
        assert_eq!(Severity::High.to_string(), "high");
        assert_eq!(Severity::Critical.to_string(), "critical");
    }
}
