use once_cell::sync::Lazy;
use regex::Regex;

static CREDENTIAL_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // API Keys
        Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[=:]\s*['"]?[a-zA-Z0-9_-]{16,}"#).unwrap(),
        Regex::new(r#"(?i)(secret[_-]?key|secretkey)\s*[=:]\s*['"]?[a-zA-Z0-9_-]{16,}"#).unwrap(),
        Regex::new(r#"(?i)(access[_-]?token|accesstoken)\s*[=:]\s*['"]?[a-zA-Z0-9_-]{16,}"#)
            .unwrap(),
        Regex::new(r#"(?i)(private[_-]?key|privatekey)\s*[=:]\s*['"]?[a-zA-Z0-9_-]{16,}"#).unwrap(),
        // Passwords
        Regex::new(r#"(?i)(password|passwd|pwd)\s*[=:]\s*['"]?[^\s'"]{6,}"#).unwrap(),
        Regex::new(r"(?i)bearer\s+[a-zA-Z0-9_.+-]+").unwrap(),
        // GitHub tokens
        Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(),
        Regex::new(r"gho_[a-zA-Z0-9]{36}").unwrap(),
        Regex::new(r"github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59}").unwrap(),
        // OpenAI
        Regex::new(r"sk-[a-zA-Z0-9]{48}").unwrap(),
        // Stripe
        Regex::new(r"sk_live_[a-zA-Z0-9]{24,}").unwrap(),
        Regex::new(r"pk_live_[a-zA-Z0-9]{24,}").unwrap(),
        Regex::new(r"rk_live_[a-zA-Z0-9]{24,}").unwrap(),
        // AWS
        Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        Regex::new(r#"(?i)aws[_-]?secret[_-]?access[_-]?key\s*[=:]\s*['"]?[a-zA-Z0-9/+=]{40}"#)
            .unwrap(),
        // Database connections
        Regex::new(r"(?i)mongodb(\+srv)?://[^\s]+").unwrap(),
        Regex::new(r"(?i)postgres(ql)?://[^\s]+").unwrap(),
        Regex::new(r"(?i)mysql://[^\s]+").unwrap(),
        Regex::new(r"(?i)redis://[^\s]+").unwrap(),
        // Private keys
        Regex::new(r"-----BEGIN (RSA |DSA |EC |OPENSSH )?PRIVATE KEY-----").unwrap(),
        // Email:password combos
        Regex::new(r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@]{4,}").unwrap(),
        // Slack
        Regex::new(r"xox[baprs]-[0-9]{10,}-[a-zA-Z0-9-]+").unwrap(),
        // Discord
        Regex::new(r#"(?i)discord[_-]?token\s*[=:]\s*['"]?[a-zA-Z0-9_.+-]+"#).unwrap(),
        Regex::new(r"[MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27}").unwrap(),
        // Twilio
        Regex::new(r#"(?i)twilio[_-]?(sid|token|key)\s*[=:]\s*['"]?[a-zA-Z0-9]+"#).unwrap(),
        Regex::new(r"AC[a-f0-9]{32}").unwrap(),
        // SendGrid
        Regex::new(r"SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}").unwrap(),
        // Webhooks
        Regex::new(r#"(?i)webhook[_-]?url\s*[=:]\s*['"]?https?://[^\s'"]+"#).unwrap(),
        // Firebase
        Regex::new(r"AIza[0-9A-Za-z_-]{35}").unwrap(),
        // Google
        Regex::new(r"ya29\.[0-9A-Za-z_-]+").unwrap(),
        // PayPal
        Regex::new(r#"(?i)paypal[_-]?(client[_-]?)?(id|secret)\s*[=:]\s*['"]?[a-zA-Z0-9-]+"#)
            .unwrap(),
        // Mailchimp
        Regex::new(r"[a-f0-9]{32}-us[0-9]{1,2}").unwrap(),
        // JWT tokens
        Regex::new(r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*").unwrap(),
        // Generic secrets in config
        Regex::new(r#"(?i)(client[_-]?secret|app[_-]?secret)\s*[=:]\s*['"]?[a-zA-Z0-9_-]{16,}"#)
            .unwrap(),
        // Heroku
        Regex::new(r#"(?i)heroku[_-]?api[_-]?key\s*[=:]\s*['"]?[a-f0-9-]+"#).unwrap(),
        // DigitalOcean
        Regex::new(r"dop_v1_[a-f0-9]{64}").unwrap(),
        // npm tokens
        Regex::new(r"npm_[a-zA-Z0-9]{36}").unwrap(),
        // Shopify
        Regex::new(r"shpat_[a-fA-F0-9]{32}").unwrap(),
        Regex::new(r"shpss_[a-fA-F0-9]{32}").unwrap(),
    ]
});

pub fn contains_credentials(content: &str) -> bool {
    CREDENTIAL_PATTERNS.iter().any(|re| re.is_match(content))
}

pub fn is_credential_related_title(title: &str) -> bool {
    let lower = title.to_lowercase();
    lower.contains("password")
        || lower.contains("credential")
        || lower.contains("api key")
        || lower.contains("apikey")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("auth")
        || lower.contains("login")
        || lower.contains("database")
        || lower.contains("config")
        || lower.contains("env")
        || lower.contains(".env")
        || lower.contains("leak")
}
