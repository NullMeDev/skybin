use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub credential_type: String,
    pub credential: String,
    pub status: ValidationStatus,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationStatus {
    Valid,
    Invalid,
    RateLimited,
    Unknown,
    Error,
}

impl std::fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationStatus::Valid => write!(f, "valid"),
            ValidationStatus::Invalid => write!(f, "invalid"),
            ValidationStatus::RateLimited => write!(f, "rate_limited"),
            ValidationStatus::Unknown => write!(f, "unknown"),
            ValidationStatus::Error => write!(f, "error"),
        }
    }
}

pub struct CredentialValidator;

impl CredentialValidator {
    pub fn new() -> Self {
        CredentialValidator
    }

    pub async fn validate(&self, credential: &str) -> ValidationResult {
        // Detect credential type and validate accordingly
        if let Some(result) = self.validate_github_token(credential).await {
            return result;
        }
        if let Some(result) = self.validate_aws_key(credential).await {
            return result;
        }
        if let Some(result) = self.validate_stripe_key(credential).await {
            return result;
        }
        if let Some(result) = self.validate_openai_key(credential).await {
            return result;
        }
        if let Some(result) = self.validate_discord_token(credential).await {
            return result;
        }

        ValidationResult {
            credential_type: "unknown".to_string(),
            credential: credential.chars().take(20).collect::<String>() + "...",
            status: ValidationStatus::Unknown,
            details: Some("Could not identify credential type".to_string()),
        }
    }

    async fn validate_github_token(&self, credential: &str) -> Option<ValidationResult> {
        let patterns = [
            Regex::new(r"^ghp_[a-zA-Z0-9]{36}$").unwrap(),
            Regex::new(r"^gho_[a-zA-Z0-9]{36}$").unwrap(),
            Regex::new(r"^github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59}$").unwrap(),
        ];

        if !patterns.iter().any(|p| p.is_match(credential)) {
            return None;
        }

        let client = reqwest::Client::new();
        match client
            .get("https://api.github.com/user")
            .header("Authorization", format!("token {}", credential))
            .header("User-Agent", "SkyBin-Validator/1.0")
            .send()
            .await
        {
            Ok(resp) => {
                let status = match resp.status().as_u16() {
                    200 => ValidationStatus::Valid,
                    401 => ValidationStatus::Invalid,
                    403 => ValidationStatus::RateLimited,
                    _ => ValidationStatus::Unknown,
                };
                Some(ValidationResult {
                    credential_type: "github_token".to_string(),
                    credential: format!(
                        "{}...{}",
                        &credential[..8],
                        &credential[credential.len() - 4..]
                    ),
                    status,
                    details: Some(format!("HTTP {}", resp.status())),
                })
            }
            Err(e) => Some(ValidationResult {
                credential_type: "github_token".to_string(),
                credential: format!("{}...", &credential[..8]),
                status: ValidationStatus::Error,
                details: Some(e.to_string()),
            }),
        }
    }

    async fn validate_aws_key(&self, credential: &str) -> Option<ValidationResult> {
        let pattern = Regex::new(r"^AKIA[0-9A-Z]{16}$").unwrap();
        if !pattern.is_match(credential) {
            return None;
        }

        // AWS keys require secret key to validate - just return format check
        Some(ValidationResult {
            credential_type: "aws_access_key".to_string(),
            credential: format!(
                "{}...{}",
                &credential[..8],
                &credential[credential.len() - 4..]
            ),
            status: ValidationStatus::Unknown,
            details: Some("AWS keys require secret key for full validation".to_string()),
        })
    }

    async fn validate_stripe_key(&self, credential: &str) -> Option<ValidationResult> {
        let patterns = [
            Regex::new(r"^sk_live_[a-zA-Z0-9]{24,}$").unwrap(),
            Regex::new(r"^pk_live_[a-zA-Z0-9]{24,}$").unwrap(),
            Regex::new(r"^rk_live_[a-zA-Z0-9]{24,}$").unwrap(),
        ];

        if !patterns.iter().any(|p| p.is_match(credential)) {
            return None;
        }

        let client = reqwest::Client::new();
        match client
            .get("https://api.stripe.com/v1/balance")
            .header("Authorization", format!("Bearer {}", credential))
            .send()
            .await
        {
            Ok(resp) => {
                let status = match resp.status().as_u16() {
                    200 => ValidationStatus::Valid,
                    401 => ValidationStatus::Invalid,
                    429 => ValidationStatus::RateLimited,
                    _ => ValidationStatus::Unknown,
                };
                Some(ValidationResult {
                    credential_type: "stripe_key".to_string(),
                    credential: format!(
                        "{}...{}",
                        &credential[..12],
                        &credential[credential.len() - 4..]
                    ),
                    status,
                    details: Some(format!("HTTP {}", resp.status())),
                })
            }
            Err(e) => Some(ValidationResult {
                credential_type: "stripe_key".to_string(),
                credential: format!("{}...", &credential[..12]),
                status: ValidationStatus::Error,
                details: Some(e.to_string()),
            }),
        }
    }

    async fn validate_openai_key(&self, credential: &str) -> Option<ValidationResult> {
        let pattern = Regex::new(r"^sk-[a-zA-Z0-9]{48}$").unwrap();
        if !pattern.is_match(credential) {
            return None;
        }

        let client = reqwest::Client::new();
        match client
            .get("https://api.openai.com/v1/models")
            .header("Authorization", format!("Bearer {}", credential))
            .send()
            .await
        {
            Ok(resp) => {
                let status = match resp.status().as_u16() {
                    200 => ValidationStatus::Valid,
                    401 => ValidationStatus::Invalid,
                    429 => ValidationStatus::RateLimited,
                    _ => ValidationStatus::Unknown,
                };
                Some(ValidationResult {
                    credential_type: "openai_key".to_string(),
                    credential: format!(
                        "{}...{}",
                        &credential[..8],
                        &credential[credential.len() - 4..]
                    ),
                    status,
                    details: Some(format!("HTTP {}", resp.status())),
                })
            }
            Err(e) => Some(ValidationResult {
                credential_type: "openai_key".to_string(),
                credential: format!("{}...", &credential[..8]),
                status: ValidationStatus::Error,
                details: Some(e.to_string()),
            }),
        }
    }

    async fn validate_discord_token(&self, credential: &str) -> Option<ValidationResult> {
        let pattern =
            Regex::new(r"^[MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27}$").unwrap();
        if !pattern.is_match(credential) {
            return None;
        }

        let client = reqwest::Client::new();
        match client
            .get("https://discord.com/api/v10/users/@me")
            .header("Authorization", credential)
            .send()
            .await
        {
            Ok(resp) => {
                let status = match resp.status().as_u16() {
                    200 => ValidationStatus::Valid,
                    401 => ValidationStatus::Invalid,
                    429 => ValidationStatus::RateLimited,
                    _ => ValidationStatus::Unknown,
                };
                Some(ValidationResult {
                    credential_type: "discord_token".to_string(),
                    credential: format!(
                        "{}...{}",
                        &credential[..8],
                        &credential[credential.len() - 4..]
                    ),
                    status,
                    details: Some(format!("HTTP {}", resp.status())),
                })
            }
            Err(e) => Some(ValidationResult {
                credential_type: "discord_token".to_string(),
                credential: format!("{}...", &credential[..8]),
                status: ValidationStatus::Error,
                details: Some(e.to_string()),
            }),
        }
    }
}

impl Default for CredentialValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_status_display() {
        assert_eq!(format!("{}", ValidationStatus::Valid), "valid");
        assert_eq!(format!("{}", ValidationStatus::Invalid), "invalid");
    }
}
