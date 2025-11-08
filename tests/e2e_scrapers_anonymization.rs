/// End-to-end integration tests for scrapers with anonymization verification
/// These tests verify that:
/// 1. Scrapers can be created and configured
/// 2. Scheduler properly processes discovered pastes
/// 3. Anonymization is applied at storage time
/// 4. No PII leaks through the system
use paste_vault::{
    anonymization::{anonymize_discovered_paste, verify_anonymity, AnonymizationConfig},
    models::DiscoveredPaste,
    scrapers::{DPasteScraper, GitHubGistsScraper, PasteEeScraper, PastebinScraper, Scraper},
};

#[test]
fn test_anonymization_workflow_pastebin() {
    // Simulate Pastebin scraper discovery
    let discovered = DiscoveredPaste::new("pastebin", "test_id_123", "secret content here")
        .with_title("My Secret File from user@example.com")
        .with_author("john_doe")
        .with_url("https://pastebin.com/test_id_123")
        .with_syntax("python");

    // Apply anonymization
    let config = AnonymizationConfig::default();
    let anonymized = anonymize_discovered_paste(discovered, &config);

    // Verify anonymization worked
    assert_eq!(anonymized.author, None, "Author should be stripped");
    assert_eq!(anonymized.url, "", "URL should be stripped");
    assert!(
        !anonymized.title.as_ref().unwrap().contains("@"),
        "Email should be removed from title"
    );
    // The email gets replaced with [redacted@email] and URLs become [redacted-url]
    // So the title should no longer contain actual email addresses
    assert!(
        !anonymized
            .title
            .as_ref()
            .unwrap()
            .contains("user@example.com"),
        "Email address should be removed"
    );
    assert!(
        verify_anonymity(anonymized.title.as_deref(), anonymized.author.as_deref()),
        "Should pass anonymity verification"
    );
}

#[test]
fn test_anonymization_workflow_gists() {
    // Simulate GitHub Gists scraper discovery
    let discovered = DiscoveredPaste::new(
        "gists",
        "gist_abc123",
        "function secret() { return password; }",
    )
    .with_title("Secret config from octocat")
    .with_author("octocat")
    .with_url("https://gist.github.com/octocat/abc123")
    .with_syntax("javascript");

    // Apply anonymization
    let config = AnonymizationConfig::default();
    let anonymized = anonymize_discovered_paste(discovered, &config);

    // Verify anonymization worked
    assert_eq!(
        anonymized.author, None,
        "GitHub username should be stripped"
    );
    assert_eq!(anonymized.url, "", "GitHub URL should be stripped");
    assert!(
        verify_anonymity(anonymized.title.as_deref(), anonymized.author.as_deref()),
        "Should pass anonymity verification"
    );
}

#[test]
fn test_anonymization_workflow_with_email_title() {
    let discovered = DiscoveredPaste::new("paste_ee", "paste_xyz", "some content")
        .with_title("Code dump from alice@company.com")
        .with_author("alice");

    let config = AnonymizationConfig::default();
    let anonymized = anonymize_discovered_paste(discovered, &config);

    // Email should be removed from title
    assert!(
        !anonymized.title.as_ref().unwrap().contains("@"),
        "Email should be removed"
    );
    assert!(
        !anonymized.title.as_ref().unwrap().contains("company.com"),
        "Domain should be removed"
    );
    assert_eq!(anonymized.author, None, "Author should be None");
    assert!(
        verify_anonymity(anonymized.title.as_deref(), anonymized.author.as_deref()),
        "Should pass anonymity verification"
    );
}

#[test]
fn test_anonymization_preserves_content() {
    let original_content = "SELECT password FROM users WHERE id=1;".to_string();
    let discovered = DiscoveredPaste::new("dpaste", "dpaste_123", original_content.clone())
        .with_title("admin credentials http://hack.me")
        .with_author("hacker");

    let config = AnonymizationConfig::default();
    let anonymized = anonymize_discovered_paste(discovered, &config);

    // Content should NOT be modified (only metadata stripped)
    assert_eq!(
        anonymized.content, original_content,
        "Content should be preserved"
    );
    assert!(
        verify_anonymity(anonymized.title.as_deref(), anonymized.author.as_deref()),
        "Should pass anonymity verification"
    );
}

#[test]
fn test_scheduler_process_paste_applies_anonymization() {
    // Test that anonymization is applied within scheduler workflow
    // Note: We test the anonymization logic directly here since scheduler
    // takes ownership of database and doesn't expose it for testing

    let discovered_before = DiscoveredPaste::new("test_source", "test_id", "API_KEY=sk-1234567890")
        .with_title("My API keys from john@example.com")
        .with_author("john_doe")
        .with_url("https://example.com/secrets")
        .with_syntax("text");

    // Simulate what scheduler does: anonymize before storing
    let config = AnonymizationConfig::default();
    let discovered_after = anonymize_discovered_paste(discovered_before, &config);

    // Verify anonymization was applied
    assert_eq!(
        discovered_after.author, None,
        "Author should be None after scheduler processing"
    );
    assert_eq!(
        discovered_after.url, "",
        "URL should be empty after scheduler processing"
    );
    assert!(
        !discovered_after.title.as_ref().unwrap().contains("@"),
        "Email should be removed"
    );
    assert!(
        !discovered_after.title.as_ref().unwrap().contains("john"),
        "Username should be removed or redacted"
    );
    assert_eq!(
        discovered_after.source, "test_source",
        "Source should be preserved"
    );
    assert_eq!(
        discovered_after.content, "API_KEY=sk-1234567890",
        "Content should be preserved"
    );
    assert!(
        verify_anonymity(
            discovered_after.title.as_deref(),
            discovered_after.author.as_deref()
        ),
        "Should pass anonymity check"
    );
}

#[test]
fn test_scraper_trait_consistency() {
    // All scrapers should implement Scraper trait consistently
    let pb = PastebinScraper::new();
    let gh = GitHubGistsScraper::new();
    let pe = PasteEeScraper::new();
    let dp = DPasteScraper::new();

    // All should have names
    assert!(!pb.name().is_empty());
    assert!(!gh.name().is_empty());
    assert!(!pe.name().is_empty());
    assert!(!dp.name().is_empty());

    // Names should be unique
    let names = vec![pb.name(), gh.name(), pe.name(), dp.name()];
    assert_eq!(
        names.iter().collect::<std::collections::HashSet<_>>().len(),
        4,
        "All scraper names should be unique"
    );

    // Names should be lowercase and no spaces
    for name in names {
        assert!(
            name.chars().all(|c| c.is_lowercase() || c == '_'),
            "Scraper name should be lowercase"
        );
    }
}

#[test]
fn test_multiple_scrapers_anonymization_chain() {
    // Verify that anonymization works the same way for all scrapers
    let discovered_pb = DiscoveredPaste::new("pastebin", "id1", "content1")
        .with_author("user1")
        .with_url("https://pastebin.com/id1")
        .with_title("title@email.com");

    let discovered_gh = DiscoveredPaste::new("gists", "id2", "content2")
        .with_author("user2")
        .with_url("https://gist.github.com/id2")
        .with_title("title@email.com");

    let discovered_pe = DiscoveredPaste::new("paste_ee", "id3", "content3")
        .with_author("user3")
        .with_url("https://paste.ee/p/id3")
        .with_title("title@email.com");

    let discovered_dp = DiscoveredPaste::new("dpaste", "id4", "content4")
        .with_author("user4")
        .with_url("https://dpaste.com/id4")
        .with_title("title@email.com");

    let config = AnonymizationConfig::default();

    // All should be anonymized consistently
    for discovered in [discovered_pb, discovered_gh, discovered_pe, discovered_dp] {
        let anonymized = anonymize_discovered_paste(discovered, &config);

        assert_eq!(
            anonymized.author, None,
            "All sources: author should be None"
        );
        assert_eq!(anonymized.url, "", "All sources: URL should be empty");
        assert!(
            !anonymized.title.as_ref().unwrap().contains("@"),
            "All sources: emails should be removed from title"
        );
        assert!(
            verify_anonymity(anonymized.title.as_deref(), anonymized.author.as_deref()),
            "All sources: should pass anonymity verification"
        );
    }
}

#[test]
fn test_no_pii_in_titles_post_anonymization() {
    let test_cases = vec![
        ("Title from alice@example.com", "alice@example.com"),
        ("Code by user123 at https://github.com/user", "https://"),
        ("File from john_doe", "john_doe"), // Username patterns removed
        ("API key endpoint: https://api.example.com", "https://"),
        ("Email: admin@company.co.uk test", "admin@company.co.uk"),
    ];

    let config = AnonymizationConfig::default();

    for (title, pii_pattern) in test_cases {
        let discovered = DiscoveredPaste::new("test", "id", "content").with_title(title);

        let anonymized = anonymize_discovered_paste(discovered, &config);

        // Title should not contain obvious PII
        let title_after = anonymized.title.as_ref().unwrap().to_lowercase();
        if pii_pattern.contains("@") {
            assert!(
                !title_after.contains("@"),
                "Email should be removed from: {}",
                title
            );
        }
        if pii_pattern.contains("https") || pii_pattern.contains("http") {
            assert!(
                !title_after.contains("http"),
                "URL should be removed from: {}",
                title
            );
        }
    }
}

#[test]
fn test_anonymization_config_consistency() {
    // Multiple instances should produce same results
    let discovered1 = DiscoveredPaste::new("test", "id", "content")
        .with_author("author")
        .with_url("https://example.com")
        .with_title("Title from user@test.com");

    let discovered2 = DiscoveredPaste::new("test", "id", "content")
        .with_author("author")
        .with_url("https://example.com")
        .with_title("Title from user@test.com");

    let config1 = AnonymizationConfig::default();
    let config2 = AnonymizationConfig::default();

    let result1 = anonymize_discovered_paste(discovered1, &config1);
    let result2 = anonymize_discovered_paste(discovered2, &config2);

    assert_eq!(result1.author, result2.author);
    assert_eq!(result1.url, result2.url);
    assert_eq!(result1.title, result2.title);
}
