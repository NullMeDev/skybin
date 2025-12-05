//! Secret Extractor Module
//!
//! Extracts, categorizes, and deduplicates secrets from text content.
//! Writes categorized secrets to respective output files.
//! Mirrors the Python credential_extractor for consistent behavior.

use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Output directory for categorized secrets
const DEFAULT_OUTPUT_DIR: &str = "/opt/skybin/extracted_secrets";

/// Database path for deduplication
const DEFAULT_DB_PATH: &str = "/opt/skybin/pastevault.db";

/// Extracted secret with metadata
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedSecret {
    pub secret_type: String,
    pub category: String,
    pub value: String,
    pub line_number: usize,
}

impl ExtractedSecret {
    /// Generate unique hash for deduplication
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", self.secret_type, self.value));
        format!("{:x}", hasher.finalize())
    }
}

/// Result of credential extraction
#[derive(Debug, Default)]
pub struct ExtractionResult {
    pub secrets: Vec<ExtractedSecret>,
    pub new_secrets: Vec<ExtractedSecret>,
    pub duplicate_secrets: Vec<ExtractedSecret>,
    pub excluded_secrets: Vec<ExtractedSecret>,
    pub categories: HashMap<String, Vec<ExtractedSecret>>,
}

impl ExtractionResult {
    pub fn total_count(&self) -> usize {
        self.secrets.len()
    }

    pub fn new_count(&self) -> usize {
        self.new_secrets.len()
    }
}

// =============================================================================
// CATEGORIZED SECRET PATTERNS
// =============================================================================

/// Pattern definition
struct SecretPattern {
    name: &'static str,
    category: &'static str,
    pattern: &'static str,
}

static PATTERN_DEFS: &[SecretPattern] = &[
    // Cloud Provider Keys
    SecretPattern { name: "AWS_Access_Key", category: "AWS_Keys", pattern: r"\b(AKIA[0-9A-Z]{16})\b" },
    SecretPattern { name: "Google_API_Key", category: "GCP_Keys", pattern: r"\b(AIza[0-9A-Za-z_-]{35})\b" },
    SecretPattern { name: "Google_OAuth_Token", category: "GCP_Keys", pattern: r"\b(ya29\.[0-9A-Za-z_-]+)\b" },
    SecretPattern { name: "DO_Personal_Token", category: "DigitalOcean_Keys", pattern: r"\b(dop_v1_[a-f0-9]{64})\b" },
    
    // Developer Platform Tokens
    SecretPattern { name: "GitHub_PAT", category: "GitHub_Tokens", pattern: r"\b(ghp_[a-zA-Z0-9]{36})\b" },
    SecretPattern { name: "GitHub_OAuth", category: "GitHub_Tokens", pattern: r"\b(gho_[a-zA-Z0-9]{36})\b" },
    SecretPattern { name: "GitHub_Fine_Grained", category: "GitHub_Tokens", pattern: r"\b(github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59})\b" },
    SecretPattern { name: "GitLab_PAT", category: "GitLab_Tokens", pattern: r"\b(glpat-[a-zA-Z0-9_-]{20})\b" },
    SecretPattern { name: "NPM_Token", category: "NPM_Tokens", pattern: r"\b(npm_[A-Za-z0-9]{36})\b" },
    
    // AI/ML API Keys
    SecretPattern { name: "OpenAI_API_Key", category: "OpenAI_Keys", pattern: r"\b(sk-[a-zA-Z0-9]{48})\b" },
    SecretPattern { name: "OpenAI_Project_Key", category: "OpenAI_Keys", pattern: r"\b(sk-proj-[a-zA-Z0-9_-]{80,})\b" },
    SecretPattern { name: "HF_Token", category: "Huggingface_Tokens", pattern: r"\b(hf_[a-zA-Z0-9]{34})\b" },
    
    // Communication Platform Tokens
    SecretPattern { name: "Discord_Bot_Token", category: "Discord_Tokens", pattern: r"\b([MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27})\b" },
    SecretPattern { name: "Slack_Bot_Token", category: "Slack_Tokens", pattern: r"\b(xoxb-[0-9]{10,}-[0-9]{10,}-[a-zA-Z0-9]{24})\b" },
    SecretPattern { name: "Telegram_Bot_Token", category: "Telegram_Tokens", pattern: r"\b(\d{8,10}:[A-Za-z0-9_-]{35})\b" },
    
    // Email Service Keys
    SecretPattern { name: "SendGrid_API_Key", category: "SendGrid_Keys", pattern: r"\b(SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43})\b" },
    SecretPattern { name: "Mailchimp_API_Key", category: "Mailchimp_Keys", pattern: r"\b([0-9a-f]{32}-us\d{1,2})\b" },
    SecretPattern { name: "Mailgun_API_Key", category: "Mailgun_Keys", pattern: r"\b(key-[0-9a-zA-Z]{32})\b" },
    
    // Payment Platform Keys
    SecretPattern { name: "Stripe_Live_Key", category: "Stripe_Keys", pattern: r"\b(sk_live_[0-9a-zA-Z]{24,})\b" },
    SecretPattern { name: "Stripe_Test_Key", category: "Stripe_Keys", pattern: r"\b(sk_test_[0-9a-zA-Z]{24,})\b" },
    SecretPattern { name: "Square_Access_Token", category: "Square_Keys", pattern: r"\b(sq0atp-[0-9A-Za-z_-]{22})\b" },
    
    // Database Credentials
    SecretPattern { name: "MongoDB_URI", category: "Database_URLs", pattern: r"(mongodb(?:\+srv)?://[^\s]+)" },
    SecretPattern { name: "PostgreSQL_URI", category: "Database_URLs", pattern: r"(postgres(?:ql)?://[^\s]+)" },
    SecretPattern { name: "MySQL_URI", category: "Database_URLs", pattern: r"(mysql://[^\s]+)" },
    SecretPattern { name: "Redis_URI", category: "Database_URLs", pattern: r"(redis://[^\s]+)" },
    
    // Private Keys
    SecretPattern { name: "RSA_Private_Key", category: "Private_Keys", pattern: r"(-----BEGIN RSA PRIVATE KEY-----[\s\S]+?-----END RSA PRIVATE KEY-----)" },
    SecretPattern { name: "OpenSSH_Private_Key", category: "Private_Keys", pattern: r"(-----BEGIN OPENSSH PRIVATE KEY-----[\s\S]+?-----END OPENSSH PRIVATE KEY-----)" },
    
    // JWT & OAuth
    SecretPattern { name: "JWT", category: "JWT_Tokens", pattern: r"\b(eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,})\b" },
    
    // Email:Password Combos
    SecretPattern { name: "Email_Password", category: "Email_Pass_Combos", pattern: r"([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+):([^\s@:]{4,})" },
    
    // Social Media
    SecretPattern { name: "Facebook_Access_Token", category: "Facebook_Tokens", pattern: r"\b(EAA[A-Za-z0-9]{100,})\b" },
    SecretPattern { name: "Twitter_Bearer", category: "Twitter_Tokens", pattern: r"\b(AAAAAAAAAAAAAAAAAAAAAA[A-Za-z0-9%]{40,})\b" },
];

/// Compiled patterns (lazily initialized)
static COMPILED_PATTERNS: Lazy<Vec<(usize, Regex)>> = Lazy::new(|| {
    PATTERN_DEFS.iter().enumerate()
        .filter_map(|(i, p)| Regex::new(p.pattern).ok().map(|r| (i, r)))
        .collect()
});

/// Category to filename mapping
static CATEGORY_FILES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("AWS_Keys", "AWS_Keys.txt");
    m.insert("GCP_Keys", "GCP_Keys.txt");
    m.insert("DigitalOcean_Keys", "DigitalOcean_Keys.txt");
    m.insert("GitHub_Tokens", "GitHub_Tokens.txt");
    m.insert("GitLab_Tokens", "GitLab_Tokens.txt");
    m.insert("NPM_Tokens", "NPM_Tokens.txt");
    m.insert("OpenAI_Keys", "OpenAI_Keys.txt");
    m.insert("Huggingface_Tokens", "Huggingface_Tokens.txt");
    m.insert("Discord_Tokens", "Discord_Tokens.txt");
    m.insert("Slack_Tokens", "Slack_Tokens.txt");
    m.insert("Telegram_Tokens", "Telegram_Tokens.txt");
    m.insert("SendGrid_Keys", "SendGrid_Keys.txt");
    m.insert("Mailchimp_Keys", "Mailchimp_Keys.txt");
    m.insert("Mailgun_Keys", "Mailgun_Keys.txt");
    m.insert("Stripe_Keys", "Stripe_Keys.txt");
    m.insert("Square_Keys", "Square_Keys.txt");
    m.insert("Database_URLs", "Database_URLs.txt");
    m.insert("Private_Keys", "Private_Keys.txt");
    m.insert("JWT_Tokens", "JWT_Tokens.txt");
    m.insert("Email_Pass_Combos", "Email_Pass_Combos.txt");
    m.insert("Facebook_Tokens", "Facebook_Tokens.txt");
    m.insert("Twitter_Tokens", "Twitter_Tokens.txt");
    m
});

/// Secret Extractor
pub struct SecretExtractor {
    db_path: String,
    output_dir: PathBuf,
    excluded_secrets: HashSet<String>,
}

impl SecretExtractor {
    /// Create a new extractor with default paths
    pub fn new() -> Self {
        Self::with_paths(DEFAULT_DB_PATH, DEFAULT_OUTPUT_DIR)
    }

    /// Create with custom paths
    pub fn with_paths(db_path: &str, output_dir: &str) -> Self {
        let mut extractor = SecretExtractor {
            db_path: db_path.to_string(),
            output_dir: PathBuf::from(output_dir),
            excluded_secrets: HashSet::new(),
        };
        extractor.load_excluded_secrets();
        extractor.ensure_output_dir();
        extractor.init_db();
        extractor
    }

    fn load_excluded_secrets(&mut self) {
        // Load from file if exists
        let exclusion_file = PathBuf::from("/opt/skybin/.excluded_secrets");
        if exclusion_file.exists() {
            if let Ok(content) = fs::read_to_string(&exclusion_file) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        self.excluded_secrets.insert(trimmed.to_string());
                    }
                }
            }
        }

        // Load from environment variables
        let env_vars = [
            "TELEGRAM_API_ID", "TELEGRAM_API_HASH",
            "SKYBIN_ADMIN_PASSWORD", "ADMIN_PASSWORD",
            "DATABASE_URL", "REDIS_URL",
            "AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY",
            "OPENAI_API_KEY", "GITHUB_TOKEN",
        ];
        for var in env_vars {
            if let Ok(val) = std::env::var(var) {
                self.excluded_secrets.insert(val);
            }
        }
    }

    fn ensure_output_dir(&self) {
        let _ = fs::create_dir_all(&self.output_dir);
    }

    fn init_db(&self) {
        if let Ok(conn) = Connection::open(&self.db_path) {
            let _ = conn.execute(
                "CREATE TABLE IF NOT EXISTS seen_secrets (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    secret_hash TEXT NOT NULL UNIQUE,
                    secret_type TEXT NOT NULL,
                    first_seen INTEGER NOT NULL,
                    last_seen INTEGER NOT NULL,
                    occurrence_count INTEGER DEFAULT 1,
                    source TEXT
                )",
                [],
            );
            let _ = conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_seen_secrets_hash ON seen_secrets(secret_hash)",
                [],
            );
        }
    }

    fn is_excluded(&self, value: &str) -> bool {
        self.excluded_secrets.iter().any(|ex| ex.contains(value) || value.contains(ex))
    }

    fn is_seen(&self, secret_hash: &str) -> bool {
        if let Ok(conn) = Connection::open(&self.db_path) {
            let count: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM seen_secrets WHERE secret_hash = ?",
                    params![secret_hash],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            return count > 0;
        }
        false
    }

    fn mark_seen(&self, secret: &ExtractedSecret, source: &str) {
        if let Ok(conn) = Connection::open(&self.db_path) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let hash = secret.hash();

            // Try update first
            let updated = conn
                .execute(
                    "UPDATE seen_secrets SET last_seen = ?, occurrence_count = occurrence_count + 1 WHERE secret_hash = ?",
                    params![now, hash],
                )
                .unwrap_or(0);

            // Insert if not updated
            if updated == 0 {
                let _ = conn.execute(
                    "INSERT INTO seen_secrets (secret_hash, secret_type, first_seen, last_seen, source) VALUES (?, ?, ?, ?, ?)",
                    params![hash, secret.secret_type, now, now, source],
                );
            }
        }
    }

    /// Extract all secrets from content
    pub fn extract(&self, content: &str, source: &str) -> ExtractionResult {
        let mut result = ExtractionResult::default();
        let mut seen_values: HashSet<String> = HashSet::new();
        let lines: Vec<&str> = content.lines().collect();

        for (pattern_idx, regex) in COMPILED_PATTERNS.iter() {
            let pattern_def = &PATTERN_DEFS[*pattern_idx];
            for (line_num, line) in lines.iter().enumerate() {
                for cap in regex.captures_iter(line) {
                    let value = cap.get(1).map(|m| m.as_str()).unwrap_or_else(|| cap.get(0).unwrap().as_str());

                    if value.len() < 8 || seen_values.contains(value) {
                        continue;
                    }
                    seen_values.insert(value.to_string());

                    let secret = ExtractedSecret {
                        secret_type: pattern_def.name.to_string(),
                        category: pattern_def.category.to_string(),
                        value: value.to_string(),
                        line_number: line_num + 1,
                    };

                    result.secrets.push(secret.clone());
                    result.categories
                        .entry(pattern_def.category.to_string())
                        .or_default()
                        .push(secret.clone());

                    if self.is_excluded(value) {
                        result.excluded_secrets.push(secret);
                        continue;
                    }

                    if self.is_seen(&secret.hash()) {
                        result.duplicate_secrets.push(secret);
                    } else {
                        result.new_secrets.push(secret.clone());
                        self.mark_seen(&secret, source);
                    }
                }
            }
        }

        result
    }

    /// Write new secrets to category files
    pub fn write_to_files(&self, result: &ExtractionResult) {
        if result.new_secrets.is_empty() {
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // Group by category
        let mut by_category: HashMap<String, Vec<&ExtractedSecret>> = HashMap::new();
        for secret in &result.new_secrets {
            by_category.entry(secret.category.clone()).or_default().push(secret);
        }

        for (category, secrets) in by_category {
            let filename = CATEGORY_FILES.get(category.as_str()).unwrap_or(&"Generic_Secrets.txt");
            let filepath = self.output_dir.join(filename);

            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&filepath)
            {
                let _ = writeln!(file, "\n# --- {} ---", timestamp);
                for secret in secrets {
                    let _ = writeln!(file, "{} | {}", secret.secret_type, secret.value);
                }
            }
        }
    }

    /// Generate summary string
    pub fn get_summary(&self, result: &ExtractionResult) -> String {
        if result.secrets.is_empty() {
            return String::new();
        }

        let mut parts: Vec<String> = Vec::new();
        for (category, secrets) in &result.categories {
            let new_count = secrets.iter().filter(|s| result.new_secrets.contains(s)).count();
            if new_count > 0 {
                parts.push(format!("{}x {}", new_count, category.replace('_', " ")));
            }
        }
        parts.truncate(5);
        parts.join(", ")
    }
}

impl Default for SecretExtractor {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Extract secrets and save new ones to files
pub fn extract_and_save(content: &str, source: &str) -> (ExtractionResult, String) {
    let extractor = SecretExtractor::new();
    let result = extractor.extract(content, source);
    
    if !result.new_secrets.is_empty() {
        extractor.write_to_files(&result);
    }
    
    let summary = extractor.get_summary(&result);
    (result, summary)
}

/// Generate credential summary for prepending to content
/// Returns (title, header) tuple
pub fn extract_credential_summary(content: &str, max_samples: usize) -> (String, String) {
    let extractor = SecretExtractor::new();
    let result = extractor.extract(content, "rust");
    
    if !result.new_secrets.is_empty() {
        extractor.write_to_files(&result);
    }
    
    if result.secrets.is_empty() {
        return (String::new(), String::new());
    }

    // Build title
    let mut title_parts: Vec<String> = Vec::new();
    for (category, secrets) in &result.categories {
        if !secrets.is_empty() {
            title_parts.push(format!("{}x {}", secrets.len(), category.replace('_', " ")));
        }
    }
    title_parts.truncate(4);
    let title = title_parts.join(", ");

    // Build header
    let mut header_lines = vec![
        "=".repeat(60),
        "CREDENTIAL SUMMARY".to_string(),
        "=".repeat(60),
        format!("Total: {} secrets ({} new, {} seen before)",
            result.total_count(), result.new_count(), result.duplicate_secrets.len()),
        String::new(),
    ];

    for (category, secrets) in &result.categories {
        if !secrets.is_empty() {
            header_lines.push(format!("{} ({} total):", category.replace('_', " ").to_uppercase(), secrets.len()));
            for secret in secrets.iter().take(max_samples) {
                let masked = if secret.value.len() > 20 {
                    format!("{}...{}", &secret.value[..8], &secret.value[secret.value.len()-8..])
                } else if secret.value.len() > 8 {
                    format!("{}...{}", &secret.value[..4], &secret.value[secret.value.len()-4..])
                } else {
                    secret.value.clone()
                };
                header_lines.push(format!("  - {}: {}", secret.secret_type, masked));
            }
            if secrets.len() > max_samples {
                header_lines.push(format!("  ... and {} more", secrets.len() - max_samples));
            }
            header_lines.push(String::new());
        }
    }

    header_lines.extend([
        "=".repeat(60),
        "                    FULL CONTENT BELOW".to_string(),
        "-".repeat(60),
        String::new(),
    ]);

    (title, header_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_aws_key() {
        let extractor = SecretExtractor::with_paths(":memory:", "/tmp/test_secrets");
        let content = "My AWS key is AKIAIOSFODNN7EXAMPLE here";
        let result = extractor.extract(content, "test");
        assert!(!result.secrets.is_empty());
        assert_eq!(result.secrets[0].category, "AWS_Keys");
    }

    #[test]
    fn test_extract_github_token() {
        let extractor = SecretExtractor::with_paths(":memory:", "/tmp/test_secrets");
        // GitHub PAT format: ghp_ followed by exactly 36 alphanumeric chars
        // abcdefghijklmnopqrstuvwxyz = 26 chars + 0123456789 = 10 chars = 36 total
        let content = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let result = extractor.extract(content, "test");
        assert!(!result.secrets.is_empty(), "Expected to find GitHub token in: {}", content);
        assert_eq!(result.secrets[0].category, "GitHub_Tokens");
    }

    #[test]
    fn test_extract_email_password() {
        let extractor = SecretExtractor::with_paths(":memory:", "/tmp/test_secrets");
        let content = "test@example.com:password123";
        let result = extractor.extract(content, "test");
        assert!(!result.secrets.is_empty());
        assert_eq!(result.secrets[0].category, "Email_Pass_Combos");
    }

    #[test]
    fn test_deduplication_hash() {
        let secret = ExtractedSecret {
            secret_type: "AWS_Access_Key".to_string(),
            category: "AWS_Keys".to_string(),
            value: "AKIAIOSFODNN7EXAMPLE".to_string(),
            line_number: 1,
        };
        let hash1 = secret.hash();
        let hash2 = secret.hash();
        assert_eq!(hash1, hash2);
    }
}
