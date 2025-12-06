use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Session token TTL - 24 hours
const SESSION_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Admin authentication manager
pub struct AdminAuth {
    /// SHA256 hash of the admin password
    password_hash: String,
    /// Active session tokens mapped to their creation time
    sessions: RwLock<HashMap<String, Instant>>,
}

impl AdminAuth {
    /// Create new admin auth with hashed password
    pub fn new(password: &str) -> Self {
        let hash = Self::hash_password(password);
        AdminAuth {
            password_hash: hash,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Hash a password with SHA256
    fn hash_password(password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Generate a secure session token
    fn generate_token() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        hex::encode(bytes)
    }

    /// Verify password and create session
    pub fn login(&self, password: &str) -> Option<String> {
        let hash = Self::hash_password(password);
        if hash == self.password_hash {
            let token = Self::generate_token();
            if let Ok(mut sessions) = self.sessions.write() {
                // Clean up expired sessions while we have the lock
                sessions.retain(|_, created| created.elapsed() < SESSION_TTL);
                sessions.insert(token.clone(), Instant::now());
            }
            Some(token)
        } else {
            None
        }
    }

    /// Verify a session token is valid (not expired)
    pub fn verify_token(&self, token: &str) -> bool {
        if let Ok(sessions) = self.sessions.read() {
            if let Some(created) = sessions.get(token) {
                return created.elapsed() < SESSION_TTL;
            }
        }
        false
    }

    /// Logout and invalidate token
    pub fn logout(&self, token: &str) {
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.remove(token);
        }
    }

    /// Cleanup expired sessions (called periodically)
    pub fn cleanup_expired(&self) {
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.retain(|_, created| created.elapsed() < SESSION_TTL);
        }
    }

    /// Extract token from Authorization header
    pub fn extract_token(auth_header: Option<&str>) -> Option<&str> {
        auth_header.and_then(|h| h.strip_prefix("Bearer "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_logout() {
        let auth = AdminAuth::new("testpassword123");

        // Wrong password
        assert!(auth.login("wrongpassword").is_none());

        // Correct password
        let token = auth.login("testpassword123").unwrap();
        assert!(auth.verify_token(&token));

        // Logout
        auth.logout(&token);
        assert!(!auth.verify_token(&token));
    }
}
