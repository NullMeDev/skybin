use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use tracing::{info, warn, debug};

use crate::archive::ExtractedPassword;
use crate::classifier::{classify_credentials, generate_title as classifier_generate_title, generate_summary_header};
use crate::stats::SharedStats;

#[derive(Clone)]
pub struct SkybinClient {
    client: Client,
    api_url: String,
    check_duplicates: bool,
}

#[derive(Serialize)]
struct CreatePasteRequest {
    content: String,
    title: Option<String>,
    source: String,
    syntax: Option<String>,
}

#[derive(Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct PasteResponse {
    id: String,
}

#[derive(Deserialize)]
struct CheckHashResponse {
    exists: bool,
}

impl SkybinClient {
    pub fn new(api_url: &str, check_duplicates: bool) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            api_url: api_url.trim_end_matches('/').to_string(),
            check_duplicates,
        }
    }
    
    /// Compute SHA256 hash of content (same as SkyBin's normalize + hash)
    fn compute_hash(content: &str) -> String {
        // Normalize: lowercase, remove extra whitespace
        let normalized = content
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        hex::encode(hasher.finalize())
    }
    
    /// Check if content already exists in SkyBin
    pub async fn check_duplicate(&self, content: &str) -> bool {
        if !self.check_duplicates {
            return false;
        }
        
        let hash = Self::compute_hash(content);
        let url = format!("{}/api/check-hash/{}", self.api_url, hash);
        
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<ApiResponse<CheckHashResponse>>().await {
                    if let Some(check) = data.data {
                        return check.exists;
                    }
                }
                false
            }
            Err(e) => {
                debug!("Error checking hash: {}", e);
                false
            }
        }
    }
    
    /// Post password file to SkyBin
    pub async fn post_paste(
        &self,
        extracted: &ExtractedPassword,
        channel_name: &str,
        stats: &SharedStats,
    ) -> Result<String, String> {
        // Classify credentials FIRST (before dedup check)
        let service_stats = classify_credentials(&extracted.content);
        
        // Skip if no credentials found
        if service_stats.total_credentials == 0 {
            info!("  ⏭️  Skipping - no credentials found");
            stats.inc_skipped_no_password();
            return Err("No credentials found".to_string());
        }
        
        // Generate title and header
        let title = classifier_generate_title(&service_stats);
        let header = generate_summary_header(&service_stats);
        let full_content = format!("{}{}", header, extracted.content);
        
        // Check for duplicate using RAW content only (not header)
        if self.check_duplicate(&extracted.content).await {
            info!("  ⏭️  Skipping duplicate content");
            stats.inc_skipped_duplicate();
            return Err("Duplicate content".to_string());
        }
        
        let request = CreatePasteRequest {
            content: full_content,
            title: Some(title),
            source: "telegram".to_string(),
            syntax: Some("text".to_string()),
        };
        
        let url = format!("{}/api/paste", self.api_url);
        
        match self.client.post(&url)
            .json(&request)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    if let Ok(data) = resp.json::<ApiResponse<PasteResponse>>().await {
                        if data.success {
                            if let Some(paste) = data.data {
                                info!("  ✅ Posted paste: {}", paste.id);
                                stats.inc_files_posted();
                                return Ok(paste.id);
                            }
                        } else if let Some(err) = data.error {
                            // Check if it's a duplicate error
                            if err.contains("duplicate") || err.contains("already exists") {
                                stats.inc_skipped_duplicate();
                                return Err("Duplicate".to_string());
                            }
                            warn!("  ❌ API error: {}", err);
                            return Err(err);
                        }
                    }
                    Ok("unknown".to_string())
                } else {
                    let err = format!("HTTP {}", status);
                    warn!("  ❌ Post failed: {}", err);
                    Err(err)
                }
            }
            Err(e) => {
                let err = format!("Request failed: {}", e);
                warn!("  ❌ {}", err);
                Err(err)
            }
        }
    }
    
    /// Send invite link for auto-join
    pub async fn receive_invite(&self, invite: &str) -> bool {
        // This would be called by SkyBin when it finds invite links in pastes
        // For now, we just log it
        info!("Received invite link: {}", invite);
        true
    }
}

