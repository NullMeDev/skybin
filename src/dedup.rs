use std::hash::{Hash, Hasher};

/// Compute a simple 64-bit SimHash over the input text.
/// Tokenizes by unicode word boundaries and uses a basic feature hashing scheme.
pub fn simhash(text: &str) -> u64 {
    // Early return for empty/short content
    if text.len() < 16 {
        return 0;
    }

    // Build weighted vector
    let mut weights = [0i64; 64];

    // Tokenize: lowercase, split on non-alphanumerics
    for token in text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
    {
        // Simple hash for the token using a per-token hasher
        let mut h = fxhash::FxHasher64::default();
        token.hash(&mut h);
        let hv = h.finish();

        // Weight longer tokens slightly higher
        let w: i64 = 1 + ((token.len() as i64).min(10) / 5);

        // Accumulate per bit position
        for bit in 0..64 {
            if (hv >> bit) & 1 == 1 {
                weights[bit] += w;
            } else {
                weights[bit] -= w;
            }
        }
    }

    // Build the final hash
    let mut out: u64 = 0;
    for bit in 0..64 {
        if weights[bit] >= 0 {
            out |= 1u64 << bit;
        }
    }
    out
}

/// Compute Hamming distance between two 64-bit integers
pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simhash_similarity() {
        let a = "This is a small combo list: user@example.com:pass123 and other data";
        let b = "Small combo list with other data user@example.com:pass123";
        let ha = simhash(a);
        let hb = simhash(b);
        assert!(hamming(ha, hb) <= 8, "expected near duplicates, got distance {}", hamming(ha, hb));
    }

    #[test]
    fn test_simhash_difference() {
        let a = "AKIAIOSFODNN7EXAMPLE aws key embedded in content";
        let b = "-----BEGIN OPENSSH PRIVATE KEY----- totally different type";
        let ha = simhash(a);
        let hb = simhash(b);
        assert!(hamming(ha, hb) >= 8);
    }
}