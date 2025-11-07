use crate::models::PatternMatch;
use super::rules::{PatternRule, BUILTIN_PATTERNS};

/// Detector for finding sensitive patterns in paste content
#[derive(Clone)]
pub struct PatternDetector {
    patterns: Vec<PatternRule>,
}

impl PatternDetector {
    /// Create a new detector with the given patterns
    pub fn new(patterns: Vec<PatternRule>) -> Self {
        PatternDetector { patterns }
    }

    /// Load all built-in patterns
    pub fn load_all() -> Self {
        let patterns = BUILTIN_PATTERNS.values().cloned().collect();
        PatternDetector { patterns }
    }

    /// Get the count of loaded patterns
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Detect all matching patterns in content
    pub fn detect(&self, content: &str) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        for pattern in &self.patterns {
            for cap in pattern.regex.captures_iter(content) {
                let matched_text = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                
                // Extract a snippet (first 100 chars or full match if shorter)
                let snippet = if matched_text.len() > 100 {
                    format!("{}...", &matched_text[..97])
                } else {
                    matched_text.to_string()
                };

                matches.push(PatternMatch {
                    name: pattern.name.clone(),
                    snippet,
                    severity: pattern.severity.to_string(),
                });
            }
        }

        // Remove duplicates while preserving highest severity
        remove_duplicate_matches(matches)
    }

    /// Determine if content is sensitive based on detected patterns
    pub fn is_sensitive(&self, content: &str) -> bool {
        let matches = self.detect(content);
        matches.iter().any(|m| m.severity == "critical" || m.severity == "high")
    }

    /// Get highest severity level detected
    pub fn get_highest_severity(&self, content: &str) -> Option<String> {
        let matches = self.detect(content);
        if matches.is_empty() {
            return None;
        }
        
        let mut highest = matches[0].severity.clone();
        let mut highest_score = severity_score(&highest);
        
        for m in &matches[1..] {
            let score = severity_score(&m.severity);
            if score > highest_score {
                highest = m.severity.clone();
                highest_score = score;
            }
        }
        
        Some(highest)
    }

    /// Count pattern matches by severity
    pub fn count_by_severity(&self, content: &str) -> SeverityCounts {
        let matches = self.detect(content);
        let mut counts = SeverityCounts::default();

        for m in matches {
            match m.severity.as_str() {
                "critical" => counts.critical += 1,
                "high" => counts.high += 1,
                "moderate" => counts.moderate += 1,
                "low" => counts.low += 1,
                _ => {}
            }
        }

        counts
    }
}

/// Counts of matches by severity
#[derive(Debug, Clone, Default)]
pub struct SeverityCounts {
    pub critical: usize,
    pub high: usize,
    pub moderate: usize,
    pub low: usize,
}

impl SeverityCounts {
    pub fn total(&self) -> usize {
        self.critical + self.high + self.moderate + self.low
    }
}

/// Remove duplicate matches, keeping the highest severity version
fn remove_duplicate_matches(matches: Vec<PatternMatch>) -> Vec<PatternMatch> {
    use std::collections::HashMap;

    let mut seen: HashMap<String, PatternMatch> = HashMap::new();

    for m in matches {
        let key = (m.name.clone(), m.snippet.clone());
        let key_str = format!("{}-{}", key.0, key.1);

        seen.entry(key_str)
            .and_modify(|existing| {
                // Keep the higher severity
                let existing_severity = severity_score(&existing.severity);
                let new_severity = severity_score(&m.severity);
                if new_severity > existing_severity {
                    *existing = m.clone();
                }
            })
            .or_insert(m);
    }

    seen.into_values().collect()
}

/// Convert severity string to numeric score for comparison
fn severity_score(severity: &str) -> u8 {
    match severity {
        "critical" => 3,
        "high" => 2,
        "moderate" => 1,
        "low" => 0,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patterns::rules::{PatternRule, Severity};

    fn create_test_detector() -> PatternDetector {
        let patterns = vec![
            PatternRule::new("AWS Key", r"AKIA[0-9A-Z]{16}", Severity::Critical, "test").unwrap(),
            PatternRule::new("API Key", r"api_key\s*[=:]\s*\w+", Severity::High, "test").unwrap(),
        ];
        PatternDetector::new(patterns)
    }

    #[test]
    fn test_detect_aws_key() {
        let detector = create_test_detector();
        let content = "My AWS key is AKIAIOSFODNN7EXAMPLE";
        let matches = detector.detect(content);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "AWS Key");
        assert_eq!(matches[0].severity, "critical");
    }

    #[test]
    fn test_detect_multiple_patterns() {
        let detector = create_test_detector();
        let content = "AWS: AKIAIOSFODNN7EXAMPLE and api_key = secret123";
        let matches = detector.detect(content);

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_is_sensitive_critical() {
        let detector = create_test_detector();
        let content = "Found AWS key: AKIAIOSFODNN7EXAMPLE";

        assert!(detector.is_sensitive(content));
    }

    #[test]
    fn test_is_sensitive_high() {
        let detector = create_test_detector();
        let content = "Set api_key = mykey";

        assert!(detector.is_sensitive(content));
    }

    #[test]
    fn test_not_sensitive() {
        let detector = create_test_detector();
        let content = "This is a normal paste with no sensitive data";

        assert!(!detector.is_sensitive(content));
    }

    #[test]
    fn test_get_highest_severity() {
        let detector = create_test_detector();
        let content = "AWS: AKIAIOSFODNN7EXAMPLE and api_key = secret";
        let highest = detector.get_highest_severity(content);

        assert_eq!(highest, Some("critical".to_string()));
    }

    #[test]
    fn test_count_by_severity() {
        let detector = create_test_detector();
        let content = "AWS: AKIAIOSFODNN7EXAMPLE and api_key = secret";
        let counts = detector.count_by_severity(content);

        assert_eq!(counts.critical, 1);
        assert_eq!(counts.high, 1);
        assert_eq!(counts.moderate, 0);
        assert_eq!(counts.low, 0);
        assert_eq!(counts.total(), 2);
    }

    #[test]
    fn test_snippet_truncation() {
        let detector = create_test_detector();
        let long_key = "api_key = ".to_string() + &"a".repeat(200);
        let matches = detector.detect(&long_key);

        assert!(!matches.is_empty());
        assert!(matches[0].snippet.len() <= 103); // 100 + "..."
    }

    #[test]
    fn test_remove_duplicates() {
        let match1 = PatternMatch {
            name: "Key".to_string(),
            snippet: "secret123".to_string(),
            severity: "high".to_string(),
        };
        let match2 = PatternMatch {
            name: "Key".to_string(),
            snippet: "secret123".to_string(),
            severity: "critical".to_string(), // Higher severity
        };

        let combined = vec![match1, match2];
        let deduplicated = remove_duplicate_matches(combined);

        assert_eq!(deduplicated.len(), 1);
        assert_eq!(deduplicated[0].severity, "critical");
    }
}
