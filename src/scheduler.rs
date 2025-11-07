use crate::db::Database;
use crate::models::Paste;
use crate::patterns::PatternDetector;
use crate::rate_limiter::SourceRateLimiter;
use crate::hash;
use chrono::Utc;
use uuid::Uuid;
use std::time::Duration;

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
    pub fn process_paste(&mut self, discovered: crate::models::DiscoveredPaste) -> anyhow::Result<()> {
        // Anonymize the paste before storing (strip authors, URLs, etc)
        let anonymization_config = crate::anonymization::AnonymizationConfig::default();
        let discovered = crate::anonymization::anonymize_discovered_paste(discovered, &anonymization_config);
        
        // Compute content hash
        let content_hash = hash::compute_hash_normalized(&discovered.content);

        // Check for duplicate
        if self.db.get_paste_by_hash(&content_hash)?.is_some() {
            return Ok(());
        }

        // Detect patterns
        let patterns = self.detector.detect(&discovered.content);
        let is_sensitive = self.detector.is_sensitive(&discovered.content);

        // Create paste record
        let now = Utc::now().timestamp();
        let paste = Paste {
            id: Uuid::new_v4().to_string(),
            source: discovered.source,
            source_id: Some(discovered.source_id),
            title: discovered.title,
            author: discovered.author,
            content: discovered.content,
            content_hash,
            url: Some(discovered.url),
            syntax: discovered.syntax.unwrap_or_else(|| "plaintext".to_string()),
            matched_patterns: if patterns.is_empty() { None } else { Some(patterns) },
            is_sensitive,
            created_at: now,
            expires_at: now + (7 * 24 * 60 * 60), // 7-day TTL
            view_count: 0,
        };

        self.db.insert_paste(&paste)?;
        Ok(())
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
