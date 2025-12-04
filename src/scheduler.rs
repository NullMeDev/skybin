use crate::db::Database;
use crate::hash;
use crate::models::Paste;
use crate::patterns::PatternDetector;
use crate::rate_limiter::SourceRateLimiter;
use chrono::Utc;
use std::time::Duration;
use uuid::Uuid;

/// Manages scraping operations
pub struct Scheduler {
    db: Database,
    detector: PatternDetector,
    #[allow(dead_code)]
    rate_limiter: SourceRateLimiter,
    #[allow(dead_code)]
    scrape_interval: Duration,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(
        db: Database,
        detector: PatternDetector,
        rate_limiter: SourceRateLimiter,
        scrape_interval_secs: u64,
    ) -> Self {
        Scheduler {
            db,
            detector,
            rate_limiter,
            scrape_interval: Duration::from_secs(scrape_interval_secs),
        }
    }

    /// Process and store a discovered paste
    pub fn process_paste(
        &mut self,
        discovered: crate::models::DiscoveredPaste,
    ) -> anyhow::Result<()> {
        // CREDENTIAL-ONLY FILTER: Only store pastes with actual credentials
        if !Self::has_credentials(&discovered.content) {
            return Ok(());
        }

        // Anonymize the paste before storing (strip authors, URLs, etc)
        let anonymization_config = crate::anonymization::AnonymizationConfig::default();
        let discovered =
            crate::anonymization::anonymize_discovered_paste(discovered, &anonymization_config);

        // Compute content hash
        let content_hash = hash::compute_hash_normalized(&discovered.content);

        // Check for duplicate
        if self.db.get_paste_by_hash(&content_hash)?.is_some() {
            return Ok(());
        }

        // Detect patterns
        let patterns = self.detector.detect(&discovered.content);
        let is_sensitive = self.detector.is_sensitive(&discovered.content);
        
        // High-value alert: flag if any pattern has critical severity
        let high_value = patterns.iter().any(|p| p.severity == "critical");

        // Auto-generate title if missing or "Untitled"
        let title = match &discovered.title {
            Some(t) if !t.is_empty() && t.to_lowercase() != "untitled" => Some(t.clone()),
            _ => Some(crate::auto_title::generate_title(&discovered.content)),
        };

        // Create paste record
        let now = Utc::now().timestamp();
        let paste = Paste {
            id: Uuid::new_v4().to_string(),
            source: discovered.source,
            source_id: Some(discovered.source_id),
            title,
            author: discovered.author,
            content: discovered.content,
            content_hash,
            url: Some(discovered.url),
            syntax: discovered.syntax.unwrap_or_else(|| "plaintext".to_string()),
            matched_patterns: if patterns.is_empty() {
                None
            } else {
                Some(patterns)
            },
            is_sensitive,
            high_value,
            created_at: now,
            expires_at: now + (7 * 24 * 60 * 60), // 7-day TTL
            view_count: 0,
        };

        self.db.insert_paste(&paste)?;
        Ok(())
    }

    /// Check if content contains actual credentials (not just keywords)
    fn has_credentials(content: &str) -> bool {
        use crate::scrapers::credential_filter::contains_credentials;
        use regex::Regex;
        use once_cell::sync::Lazy;
        
        // Minimum length requirement
        if content.len() < 50 {
            return false;
        }
        
        let content_lower = content.to_lowercase();
        
        // Check for private keys (always accept)
        if content.contains("-----BEGIN") && content.contains("PRIVATE KEY-----") {
            return true;
        }
        
        // Check for credential patterns (API keys, tokens, etc) - accept 1+
        if contains_credentials(content) {
            return true;
        }
        
        // Check for email:password combos - accept 1+
        static EMAIL_PASS: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@]{4,}").unwrap()
        });
        if EMAIL_PASS.is_match(content) {
            return true;
        }
        
        // Check for URL:login:pass format (stealer logs) - accept 1+
        static ULP: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"https?://[^\s]+[\s\t|:]+[^\s@]+[\s\t|:]+[^\s]{4,}").unwrap()
        });
        if ULP.is_match(content) {
            return true;
        }
        
        // Check for leak keywords (need 3+ for keyword-only detection)
        let leak_keywords = [
            "leak", "leaked", "dump", "dumped", "combo", "combolist", "breach",
            "crack", "cracked", "hacked", "stolen", "exposed", "database",
            "credential", "password", "stealer", "infostealer", "redline", "raccoon",
            "netflix", "spotify", "disney", "vpn", "steam", "fortnite",
            "paypal", "crypto", "bitcoin", "wallet", "api key", "apikey",
            "token", "secret", "ssh", "ftp", "smtp", "cpanel", "rdp",
            "fresh", "valid", "checked", "hits", "email:pass", "user:pass",
            "bin", "fullz", "cvv", "ssn",
        ];
        let keyword_count = leak_keywords.iter().filter(|kw| content_lower.contains(*kw)).count();
        if keyword_count >= 3 {
            return true;
        }
        
        false
    }
    
    /// Global quality check for all pastes - DISABLED for admin moderation
    #[allow(dead_code)]
    fn passes_quality_check(content: &str) -> bool {
        let content_lower = content.to_lowercase();
        let line_count = content.lines().count();
        let content_len = content.len();
        
        // Minimum requirements
        if content_len < 100 || line_count < 3 {
            return false;
        }
        
        // Immediately pass if it has sensitive content
        if Self::has_interesting_content(&content_lower) {
            return true;
        }
        
        // Filter out pure code without sensitive content
        let code_indicators = [
            "#include", "using namespace", "int main",
            "public class", "public static void main",
            "def ", "import ", "from ", "class ",
            "function ", "const ", "let ", "var ",
            "package main", "func main",
        ];
        
        let is_code = code_indicators.iter().any(|i| content_lower.contains(i));
        
        // If it's code, require it to be substantial or have interesting content
        if is_code {
            // Require more lines for code
            if line_count < 30 {
                return false;
            }
        }
        
        // Skip content that's mostly single-character lines or gibberish
        let avg_line_len = content_len / line_count.max(1);
        if avg_line_len < 10 {
            return false;
        }
        
        true
    }
    
    /// Check if content contains interesting/sensitive patterns
    #[allow(dead_code)]
    fn has_interesting_content(content: &str) -> bool {
        let interesting_patterns = [
            // Credentials
            "password", "passwd", "pwd=", ":pass",
            "api_key", "apikey", "api-key", "api_secret",
            "token", "bearer", "auth", "oauth",
            "secret", "credential", "private",
            // Database
            "mysql://", "postgres://", "mongodb://", "redis://",
            "database", "db_host", "db_user", "db_pass",
            // Cloud
            "aws_", "azure", "gcp_", "digitalocean",
            "s3://", "bucket",
            // Email combos
            "@gmail", "@yahoo", "@outlook", "@hotmail", "@proton",
            ":password", ":pass", ":123",
            // Payment
            "stripe", "paypal", "credit", "card",
            "bitcoin", "ethereum", "wallet", "crypto",
            // Services
            "netflix", "spotify", "disney", "hulu", "hbo",
            "steam", "epic", "playstation", "xbox",
            "discord", "telegram", "slack", "twitch",
            // Infrastructure
            "ssh", "ftp", "smtp", "vpn", "proxy",
            "server", "admin", "root",
            // Security
            "hack", "exploit", "vuln", "breach", "leak", "dump",
            "phish", "malware", "backdoor",
            // Config
            ".env", "config", "settings", "credentials",
            "-----begin", "-----end",
        ];
        
        for pattern in interesting_patterns {
            if content.contains(pattern) {
                return true;
            }
        }
        
        // Check for email:password combo format
        let lines: Vec<&str> = content.lines().take(20).collect();
        let combo_count = lines.iter()
            .filter(|l| l.contains('@') && l.contains(':') && l.len() > 10 && l.len() < 200)
            .count();
        if combo_count >= 3 {
            return true;
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let db = Database::open(":memory:").unwrap();
        let detector = PatternDetector::new(vec![]);
        let limiter = SourceRateLimiter::default_jitter();

        let scheduler = Scheduler::new(db, detector, limiter, 300);
        assert_eq!(scheduler.scrape_interval, Duration::from_secs(300));
    }
}
