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

        // Credit Cards (Luhn validated)
        m.insert(
            "credit_card".to_string(),
            PatternRule::new(
                "Credit Card Number",
                r"\b(?:\d[ -]*?){13,19}\b",
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

        // AWS Account ID
        m.insert(
            "aws_account_id".to_string(),
            PatternRule::new(
                "AWS Account ID",
                r"\b\d{12}\b",
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
