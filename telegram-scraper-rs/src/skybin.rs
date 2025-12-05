use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use tracing::{info, warn, debug};

use crate::archive::ExtractedPassword;
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
        // Check for duplicate first
        if self.check_duplicate(&extracted.content).await {
            info!("  ⏭️  Skipping duplicate content");
            stats.inc_skipped_duplicate();
            return Err("Duplicate content".to_string());
        }
        
        // Generate title
        let title = generate_title(extracted, channel_name);
        
        let request = CreatePasteRequest {
            content: extracted.content.clone(),
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

/// Generate title for password file
fn generate_title(extracted: &ExtractedPassword, channel_name: &str) -> String {
    let mut parts = vec![format!("[TG] {}", channel_name)];
    
    // Detect services from content
    let services = detect_services(&extracted.content);
    if !services.is_empty() {
        parts.push(services.join(" / "));
    }
    
    // Add credential counts
    if extracted.email_pass_count > 0 {
        parts.push(format!("{} Email:Pass", extracted.email_pass_count));
    } else if extracted.url_login_count > 0 {
        parts.push(format!("{} URL:Login:Pass", extracted.url_login_count));
    } else if extracted.line_count > 10 {
        parts.push(format!("{} lines", extracted.line_count));
    }
    
    let title = parts.join(" - ");
    if title.len() > 100 {
        title[..100].to_string()
    } else {
        title
    }
}

/// Detect services mentioned in content
fn detect_services(content: &str) -> Vec<String> {
    let lower = content.to_lowercase();
    let mut services = vec![];
    
    let service_map = [
        ("twitter", "Twitter"),
        ("facebook", "Facebook"),
        ("instagram", "Instagram"),
        ("gmail", "Gmail"),
        ("outlook", "Outlook"),
        ("yahoo", "Yahoo"),
        ("netflix", "Netflix"),
        ("spotify", "Spotify"),
        ("steam", "Steam"),
        ("paypal", "PayPal"),
        ("amazon", "Amazon"),
        ("discord", "Discord"),
        ("tiktok", "TikTok"),
        ("linkedin", "LinkedIn"),
        ("github", "GitHub"),
        ("roblox", "Roblox"),
        ("fortnite", "Fortnite"),
        ("minecraft", "Minecraft"),
    ];
    
    for (keyword, name) in service_map {
        if lower.contains(keyword) && !services.contains(&name.to_string()) {
            services.push(name.to_string());
            if services.len() >= 3 {
                break;
            }
        }
    }
    
    services
}
