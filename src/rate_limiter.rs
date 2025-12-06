use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Simple rate limiter using timestamps
#[derive(Debug, Clone)]
struct SimpleRateLimiter {
    last_request: SystemTime,
    interval: Duration,
}

impl SimpleRateLimiter {
    fn new(requests_per_second: u32) -> Self {
        let interval = Duration::from_millis(1000 / requests_per_second as u64);
        SimpleRateLimiter {
            last_request: SystemTime::UNIX_EPOCH,
            interval,
        }
    }

    fn check_and_update(&mut self) -> bool {
        let now = SystemTime::now();
        if now
            .duration_since(self.last_request)
            .unwrap_or(Duration::ZERO)
            >= self.interval
        {
            self.last_request = now;
            true
        } else {
            false
        }
    }
}

/// Rate limiter for controlling request frequency per source
#[derive(Clone)]
pub struct SourceRateLimiter {
    limiters: Arc<Mutex<HashMap<String, SimpleRateLimiter>>>,
    jitter_min_ms: u64,
    jitter_max_ms: u64,
    /// Per-source request rate limits (requests per second)
    source_limits: Arc<HashMap<String, u32>>,
}

impl SourceRateLimiter {
    /// Create a new rate limiter with specified jitter range
    pub fn new(jitter_min_ms: u64, jitter_max_ms: u64) -> Self {
        SourceRateLimiter {
            limiters: Arc::new(Mutex::new(HashMap::new())),
            jitter_min_ms,
            jitter_max_ms,
            source_limits: Arc::new(HashMap::new()),
        }
    }

    /// Create with source-specific rate limits
    pub fn with_source_limits(
        jitter_min_ms: u64,
        jitter_max_ms: u64,
        source_limits: HashMap<String, u32>,
    ) -> Self {
        SourceRateLimiter {
            limiters: Arc::new(Mutex::new(HashMap::new())),
            jitter_min_ms,
            jitter_max_ms,
            source_limits: Arc::new(source_limits),
        }
    }

    /// Create with default jitter (500-5000ms)
    pub fn default_jitter() -> Self {
        SourceRateLimiter::new(500, 5000)
    }

    /// Create with default jitter and source-specific rate limits
    pub fn default_with_source_limits(source_limits: HashMap<String, u32>) -> Self {
        SourceRateLimiter::with_source_limits(500, 5000, source_limits)
    }

    /// Check if a request is allowed for the source (without blocking)
    pub fn check_rate_limit(&self, source: &str) -> bool {
        let mut limiters = self.limiters.lock().unwrap();
        // Get source-specific rate limit, default to 1 req/sec if not configured
        let rate = self.source_limits.get(source).copied().unwrap_or(1);
        limiters
            .entry(source.to_string())
            .or_insert_with(|| SimpleRateLimiter::new(rate))
            .check_and_update()
    }

    /// Wait until a request is allowed (blocking with jitter)
    pub async fn wait_rate_limit(&self, source: &str) {
        let jitter = self.random_jitter();

        loop {
            if self.check_rate_limit(source) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Add jitter delay
        tokio::time::sleep(jitter).await;
    }

    /// Get a random jitter duration within configured range
    fn random_jitter(&self) -> Duration {
        let mut rng = rand::thread_rng();
        let jitter_ms = rng.gen_range(self.jitter_min_ms..=self.jitter_max_ms);
        Duration::from_millis(jitter_ms)
    }
}

/// Exponential backoff retry strategy
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    initial_ms: u64,
    max_ms: u64,
    max_retries: usize,
    current_retry: usize,
}

impl ExponentialBackoff {
    /// Create new backoff strategy
    pub fn new(initial_ms: u64, max_ms: u64, max_retries: usize) -> Self {
        ExponentialBackoff {
            initial_ms,
            max_ms,
            max_retries,
            current_retry: 0,
        }
    }

    /// Check if we can retry
    pub fn can_retry(&self) -> bool {
        self.current_retry < self.max_retries
    }

    /// Get next backoff duration and increment retry count
    pub fn next_backoff(&mut self) -> Option<Duration> {
        if !self.can_retry() {
            return None;
        }

        let backoff_ms = std::cmp::min(
            self.initial_ms * 2u64.pow(self.current_retry as u32),
            self.max_ms,
        );

        self.current_retry += 1;
        Some(Duration::from_millis(backoff_ms))
    }

    /// Reset retry counter
    pub fn reset(&mut self) {
        self.current_retry = 0;
    }
}

// =============================================================================
// API ENDPOINT RATE LIMITERS
// =============================================================================

use std::time::Instant;

/// Sliding window rate limiter for API endpoints
/// Uses request counts per time window rather than intervals
pub struct ApiRateLimiter {
    /// Maximum requests per window
    max_requests: u32,
    /// Window duration
    window: Duration,
    /// Requests by key (typically client identifier)
    requests: Mutex<HashMap<String, Vec<Instant>>>,
}

impl ApiRateLimiter {
    /// Create a new API rate limiter
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs),
            requests: Mutex::new(HashMap::new()),
        }
    }

    /// Check if request is allowed and record it
    /// Returns true if allowed, false if rate limited
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window;

        let mut requests = match self.requests.lock() {
            Ok(r) => r,
            Err(_) => return true, // Fail open on lock error
        };

        let timestamps = requests.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove expired entries
        timestamps.retain(|t| *t > cutoff);

        if timestamps.len() >= self.max_requests as usize {
            false
        } else {
            timestamps.push(now);
            true
        }
    }

    /// Cleanup old entries to prevent memory growth
    pub fn cleanup(&self) {
        let cutoff = Instant::now() - self.window;

        if let Ok(mut requests) = self.requests.lock() {
            requests.retain(|_, timestamps| {
                timestamps.retain(|t| *t > cutoff);
                !timestamps.is_empty()
            });
        }
    }
}

/// Pre-configured rate limiters for API endpoints
pub struct ApiRateLimiters {
    /// Uploads: 10 per minute
    pub upload: ApiRateLimiter,
    /// URL submissions: 20 per minute
    pub submit_url: ApiRateLimiter,
    /// Search: 60 per minute
    pub search: ApiRateLimiter,
    /// Admin login: 5 per minute (anti-brute-force)
    pub admin_login: ApiRateLimiter,
    /// Comments: 10 per minute
    pub comments: ApiRateLimiter,
}

impl ApiRateLimiters {
    pub fn new() -> Self {
        Self {
            upload: ApiRateLimiter::new(10, 60),
            submit_url: ApiRateLimiter::new(20, 60),
            search: ApiRateLimiter::new(60, 60),
            admin_login: ApiRateLimiter::new(5, 60),
            comments: ApiRateLimiter::new(10, 60),
        }
    }

    /// Cleanup all limiters (call periodically)
    pub fn cleanup_all(&self) {
        self.upload.cleanup();
        self.submit_url.cleanup();
        self.search.cleanup();
        self.admin_login.cleanup();
        self.comments.cleanup();
    }
}

impl Default for ApiRateLimiters {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = SourceRateLimiter::new(100, 500);
        assert_eq!(limiter.jitter_min_ms, 100);
        assert_eq!(limiter.jitter_max_ms, 500);
    }

    #[test]
    fn test_default_jitter() {
        let limiter = SourceRateLimiter::default_jitter();
        assert_eq!(limiter.jitter_min_ms, 500);
        assert_eq!(limiter.jitter_max_ms, 5000);
    }

    #[test]
    fn test_jitter_range() {
        let limiter = SourceRateLimiter::new(100, 200);
        for _ in 0..100 {
            let jitter = limiter.random_jitter();
            let ms = jitter.as_millis() as u64;
            assert!(ms >= 100 && ms <= 200);
        }
    }

    #[test]
    fn test_rate_limit_check() {
        let limiter = SourceRateLimiter::new(100, 200);

        // First request should be allowed
        assert!(limiter.check_rate_limit("test"));

        // Second request immediately should be denied (1 req/sec)
        assert!(!limiter.check_rate_limit("test"));
    }

    #[test]
    fn test_multiple_sources() {
        let limiter = SourceRateLimiter::new(100, 200);

        // Different sources should have independent limits
        assert!(limiter.check_rate_limit("source1"));
        assert!(limiter.check_rate_limit("source2"));

        // Both should reject immediate second request
        assert!(!limiter.check_rate_limit("source1"));
        assert!(!limiter.check_rate_limit("source2"));
    }

    #[test]
    fn test_exponential_backoff() {
        let mut backoff = ExponentialBackoff::new(100, 1000, 3);

        assert!(backoff.can_retry());
        assert_eq!(backoff.next_backoff(), Some(Duration::from_millis(100)));
        assert_eq!(backoff.current_retry, 1);

        assert!(backoff.can_retry());
        assert_eq!(backoff.next_backoff(), Some(Duration::from_millis(200)));
        assert_eq!(backoff.current_retry, 2);

        assert!(backoff.can_retry());
        assert_eq!(backoff.next_backoff(), Some(Duration::from_millis(400)));
        assert_eq!(backoff.current_retry, 3);

        assert!(!backoff.can_retry());
        assert_eq!(backoff.next_backoff(), None);
    }

    #[test]
    fn test_exponential_backoff_max_cap() {
        let mut backoff = ExponentialBackoff::new(100, 500, 10);

        // Should cap at max_ms
        for _ in 0..5 {
            backoff.next_backoff();
        }

        let final_backoff = backoff.next_backoff().unwrap();
        assert!(final_backoff.as_millis() <= 500);
    }

    #[test]
    fn test_exponential_backoff_reset() {
        let mut backoff = ExponentialBackoff::new(100, 1000, 3);

        backoff.next_backoff();
        backoff.next_backoff();
        assert_eq!(backoff.current_retry, 2);

        backoff.reset();
        assert_eq!(backoff.current_retry, 0);
        assert!(backoff.can_retry());
    }

    #[tokio::test]
    async fn test_wait_rate_limit() {
        let limiter = SourceRateLimiter::new(10, 50);

        // Should not hang and should eventually allow request
        limiter.wait_rate_limit("test").await;
        // If we get here, the test passed
        assert!(true);
    }

    #[test]
    fn test_per_source_rate_limits() {
        use std::collections::HashMap;

        let mut limits = HashMap::new();
        limits.insert("pastebin".to_string(), 2); // 2 req/sec
        limits.insert("gists".to_string(), 5); // 5 req/sec

        let limiter = SourceRateLimiter::with_source_limits(100, 200, limits);

        // Pastebin gets 2 req/sec, gists gets 5 req/sec, unknown gets 1 req/sec
        assert!(limiter.check_rate_limit("pastebin"));
        assert!(limiter.check_rate_limit("gists"));
        assert!(limiter.check_rate_limit("unknown"));
    }

    #[test]
    fn test_default_with_source_limits() {
        use std::collections::HashMap;

        let mut limits = HashMap::new();
        limits.insert("test".to_string(), 10);

        let limiter = SourceRateLimiter::default_with_source_limits(limits);
        assert_eq!(limiter.jitter_min_ms, 500);
        assert_eq!(limiter.jitter_max_ms, 5000);
        assert_eq!(limiter.source_limits.get("test"), Some(&10));
    }
}
