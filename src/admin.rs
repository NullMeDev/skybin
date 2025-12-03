use sha2::{Sha256, Digest};
use std::sync::RwLock;
use std::collections::HashSet;

/// Admin authentication manager
pub struct AdminAuth {
    /// SHA256 hash of the admin password
    password_hash: String,
    /// Active session tokens with expiry timestamps
    sessions: RwLock<HashSet<String>>,
}

impl AdminAuth {
    /// Create new admin auth with hashed password
    pub fn new(password: &str) -> Self {
        let hash = Self::hash_password(password);
        AdminAuth {
            password_hash: hash,
            sessions: RwLock::new(HashSet::new()),
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
                sessions.insert(token.clone());
            }
            Some(token)
        } else {
            None
        }
    }

    /// Verify a session token is valid
    pub fn verify_token(&self, token: &str) -> bool {
        if let Ok(sessions) = self.sessions.read() {
            sessions.contains(token)
        } else {
            false
        }
    }

    /// Logout and invalidate token
    pub fn logout(&self, token: &str) {
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.remove(token);
        }
    }

    /// Extract token from Authorization header
    pub fn extract_token(auth_header: Option<&str>) -> Option<&str> {
        auth_header.and_then(|h| {
            if h.starts_with("Bearer ") {
                Some(&h[7..])
            } else {
                None
            }
        })
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
