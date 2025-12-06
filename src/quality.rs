use sha2::{Digest, Sha256};

/// Validate email:password combo format
pub fn validate_combo(line: &str) -> bool {
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() != 2 {
        return false;
    }
    let email = parts[0].trim();
    let pass = parts[1].trim();

    // Basic email validation
    if !email.contains('@') || !email.contains('.') {
        return false;
    }

    // Check email has valid structure
    let email_parts: Vec<&str> = email.split('@').collect();
    if email_parts.len() != 2 || email_parts[0].is_empty() || email_parts[1].is_empty() {
        return false;
    }

    // Domain must have at least one dot
    if !email_parts[1].contains('.') {
        return false;
    }

    // Password must be non-empty and reasonable length
    !pass.is_empty() && pass.len() >= 4 && pass.len() <= 128
}

/// Count valid combos in content
pub fn count_valid_combos(content: &str) -> usize {
    content.lines().filter(|line| validate_combo(line)).count()
}

/// Check if content is a combo list (>50% valid combos)
pub fn is_combo_list(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 3 {
        return false;
    }
    let valid = count_valid_combos(content);
    valid as f64 / lines.len() as f64 > 0.5
}

/// Generate SHA256 hash for deduplication
pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    // Normalize: lowercase, remove extra whitespace
    let normalized: String = content
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

/// Detect programming language from content
pub fn detect_language(content: &str) -> &'static str {
    let c = content.to_lowercase();
    let patterns = [
        // Check most specific first
        (vec!["<?php", "<?="], "php"),
        (
            vec![
                "#!/usr/bin/env python",
                "#!/usr/bin/python",
                "import ",
                "def ",
                "print(",
            ],
            "python",
        ),
        (
            vec!["#!/bin/bash", "#!/usr/bin/env bash", "#!/bin/sh"],
            "bash",
        ),
        (vec!["package main", "func main()", "import \"fmt\""], "go"),
        (vec!["fn main()", "let mut ", "impl ", "pub fn"], "rust"),
        (
            vec!["public class ", "public static void main", "System.out."],
            "java",
        ),
        (
            vec!["using System;", "namespace ", "Console.Write"],
            "csharp",
        ),
        (
            vec!["#include <", "int main(", "printf(", "cout <<"],
            "c/cpp",
        ),
        (
            vec!["function ", "const ", "let ", "var ", "=>", "console.log"],
            "javascript",
        ),
        (
            vec!["import React", "useState", "useEffect", "jsx"],
            "react",
        ),
        (vec!["<template>", "v-if", "v-for", ":class"], "vue"),
        (vec!["<!DOCTYPE html", "<html", "<head>", "<body>"], "html"),
        (
            vec![
                "SELECT ",
                "INSERT INTO",
                "CREATE TABLE",
                "UPDATE ",
                "DELETE FROM",
            ],
            "sql",
        ),
        (vec!["{\"", "\":", "[{"], "json"),
        (vec!["---\n", "apiVersion:", "kind:"], "yaml"),
        (vec!["[Unit]", "[Service]", "[Install]"], "systemd"),
        (vec!["FROM ", "RUN ", "COPY ", "ENTRYPOINT"], "dockerfile"),
    ];

    for (keywords, lang) in patterns {
        for kw in keywords {
            if c.contains(kw) {
                return lang;
            }
        }
    }

    "plaintext"
}

/// Calculate Shannon entropy (higher = more random/potentially sensitive)
pub fn calculate_entropy(content: &str) -> f64 {
    if content.is_empty() {
        return 0.0;
    }

    let mut freq = [0u64; 256];
    let len = content.len() as f64;

    for byte in content.bytes() {
        freq[byte as usize] += 1;
    }

    let mut entropy = 0.0;
    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Check if content has high entropy (likely contains secrets)
pub fn has_high_entropy(content: &str) -> bool {
    // Look for high-entropy substrings (potential keys/tokens)
    for line in content.lines() {
        let line = line.trim();
        if line.len() >= 20 && line.len() <= 200 {
            let entropy = calculate_entropy(line);
            // Entropy > 4.5 for a line suggests random/encoded data
            if entropy > 4.5 {
                return true;
            }
        }
    }
    false
}

/// Quality score for paste (0-100)
pub fn quality_score(content: &str) -> u32 {
    let mut score = 50u32; // Base score

    // Bonus for combo lists
    if is_combo_list(content) {
        score += 30;
    }

    // Bonus for high entropy
    if has_high_entropy(content) {
        score += 20;
    }

    // Bonus for length (more content = potentially more interesting)
    let lines = content.lines().count();
    if lines > 10 {
        score += 5;
    }
    if lines > 50 {
        score += 5;
    }
    if lines > 100 {
        score += 5;
    }

    // Penalty for very short content
    if lines < 5 {
        score = score.saturating_sub(20);
    }

    score.min(100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_combo() {
        assert!(validate_combo("test@example.com:password123"));
        assert!(validate_combo("user@domain.co.uk:pass"));
        assert!(!validate_combo("notanemail:password"));
        assert!(!validate_combo("test@example.com:"));
        assert!(!validate_combo("test@:password"));
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("<?php echo 'hello';"), "php");
        assert_eq!(detect_language("def main():\n    print('hi')"), "python");
        assert_eq!(
            detect_language("#include <stdio.h>\nint main() {}"),
            "c/cpp"
        );
    }

    #[test]
    fn test_entropy() {
        let low = calculate_entropy("aaaaaaaaaa");
        let high = calculate_entropy("aB3$xY9!kL");
        assert!(high > low);
    }
}
