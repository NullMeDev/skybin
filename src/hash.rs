use sha2::{Digest, Sha256};

/// Compute SHA256 hash of content for deduplication
///
/// # Arguments
/// * `content` - The paste content to hash
///
/// # Returns
/// Hex-encoded SHA256 hash string
pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Normalize content before hashing (optional preprocessing)
///
/// Currently does basic trimming, but can be extended for:
/// - Line ending normalization (CRLF vs LF)
/// - Whitespace normalization
/// - Unicode normalization
pub fn normalize_content(content: &str) -> String {
    content.trim().to_string()
}

/// Compute hash of normalized content
pub fn compute_hash_normalized(content: &str) -> String {
    let normalized = normalize_content(content);
    compute_hash(&normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let content = "hello world";
        let hash = compute_hash(content);
        // SHA256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_hash_consistency() {
        let content = "test content";
        let hash1 = compute_hash(content);
        let hash2 = compute_hash(content);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_different_content() {
        let hash1 = compute_hash("content1");
        let hash2 = compute_hash("content2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_normalize_content() {
        let content_with_whitespace = "  hello world  \n";
        let normalized = normalize_content(content_with_whitespace);
        assert_eq!(normalized, "hello world");
    }

    #[test]
    fn test_hash_normalized_consistency() {
        let content1 = "test content";
        let content2 = "  test content  \n";

        let hash1 = compute_hash_normalized(content1);
        let hash2 = compute_hash_normalized(content2);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_empty_content() {
        let hash = compute_hash("");
        // SHA256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_hash_length() {
        let hash = compute_hash("test");
        // SHA256 in hex is 64 characters
        assert_eq!(hash.len(), 64);
    }
}
