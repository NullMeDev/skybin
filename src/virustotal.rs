use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

const VIRUSTOTAL_API_URL: &str = "https://www.virustotal.com/api/v3";
const MAX_FILE_SIZE: usize = 32 * 1024 * 1024; // 32MB standard limit
const LARGE_FILE_SIZE: usize = 650 * 1024 * 1024; // 650MB with upload_url

#[derive(Debug, Serialize, Deserialize)]
struct VTUploadResponse {
    data: VTUploadData,
}

#[derive(Debug, Serialize, Deserialize)]
struct VTUploadData {
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VTAnalysisResponse {
    data: VTAnalysisData,
}

#[derive(Debug, Serialize, Deserialize)]
struct VTAnalysisData {
    attributes: VTAttributes,
}

#[derive(Debug, Serialize, Deserialize)]
struct VTAttributes {
    status: String,
    stats: VTStats,
}

#[derive(Debug, Serialize, Deserialize)]
struct VTStats {
    #[serde(default)]
    malicious: u32,
    #[serde(default)]
    suspicious: u32,
    #[serde(default)]
    undetected: u32,
    #[serde(default)]
    harmless: u32,
}

#[derive(Debug, Clone)]
pub struct VirusTotalClient {
    client: Client,
    api_key: String,
    enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub is_safe: bool,
    pub malicious_count: u32,
    pub suspicious_count: u32,
    pub total_scans: u32,
}

impl VirusTotalClient {
    pub fn new(api_key: String, enabled: bool) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
            api_key,
            enabled,
        }
    }

    /// Scan a file with VirusTotal
    /// Returns Ok(ScanResult) if scan completes successfully
    /// Returns Err if API key is missing, file too large, or scan fails
    pub async fn scan_file(&self, file_data: &[u8], filename: &str) -> Result<ScanResult> {
        if !self.enabled {
            debug!("VirusTotal scanning disabled, skipping");
            return Ok(ScanResult {
                is_safe: true,
                malicious_count: 0,
                suspicious_count: 0,
                total_scans: 0,
            });
        }

        if self.api_key.is_empty() {
            return Err(anyhow!("VirusTotal API key not configured"));
        }

        let file_size = file_data.len();

        if file_size > LARGE_FILE_SIZE {
            return Err(anyhow!("File too large for VirusTotal (max 650MB)"));
        }

        info!(
            "Scanning file {} ({:.2} MB) with VirusTotal...",
            filename,
            file_size as f64 / 1024.0 / 1024.0
        );

        // Upload file
        let analysis_id = if file_size <= MAX_FILE_SIZE {
            self.upload_small_file(file_data, filename).await?
        } else {
            self.upload_large_file(file_data, filename).await?
        };

        // Wait for analysis to complete
        self.wait_for_analysis(&analysis_id).await
    }

    /// Upload file using standard endpoint (≤32MB)
    async fn upload_small_file(&self, file_data: &[u8], filename: &str) -> Result<String> {
        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(file_data.to_vec()).file_name(filename.to_string()),
        );

        let response = self
            .client
            .post(format!("{}/files", VIRUSTOTAL_API_URL))
            .header("x-apikey", &self.api_key)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("VT upload failed: {} - {}", status, body));
        }

        let upload_response: VTUploadResponse = response.json().await?;
        Ok(upload_response.data.id)
    }

    /// Upload file using special upload URL endpoint (>32MB, ≤650MB)
    async fn upload_large_file(&self, file_data: &[u8], filename: &str) -> Result<String> {
        // Get upload URL
        let upload_url_response = self
            .client
            .get(format!("{}/files/upload_url", VIRUSTOTAL_API_URL))
            .header("x-apikey", &self.api_key)
            .send()
            .await?;

        if !upload_url_response.status().is_success() {
            return Err(anyhow!("Failed to get VT upload URL"));
        }

        let upload_url_data: serde_json::Value = upload_url_response.json().await?;
        let upload_url = upload_url_data["data"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid upload URL response"))?;

        // Upload to the provided URL
        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(file_data.to_vec()).file_name(filename.to_string()),
        );

        let response = self
            .client
            .post(upload_url)
            .header("x-apikey", &self.api_key)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("VT large file upload failed"));
        }

        let upload_response: VTUploadResponse = response.json().await?;
        Ok(upload_response.data.id)
    }

    /// Poll VirusTotal analysis status until complete
    async fn wait_for_analysis(&self, analysis_id: &str) -> Result<ScanResult> {
        const MAX_ATTEMPTS: u32 = 30;
        const POLL_INTERVAL_SECS: u64 = 10;

        for attempt in 1..=MAX_ATTEMPTS {
            debug!("Polling VT analysis status (attempt {})", attempt);

            let response = self
                .client
                .get(format!("{}/analyses/{}", VIRUSTOTAL_API_URL, analysis_id))
                .header("x-apikey", &self.api_key)
                .send()
                .await?;

            if !response.status().is_success() {
                warn!("VT analysis check failed: {}", response.status());
                sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                continue;
            }

            let analysis: VTAnalysisResponse = response.json().await?;
            let status = &analysis.data.attributes.status;

            if status == "completed" {
                let stats = &analysis.data.attributes.stats;
                let total = stats.malicious + stats.suspicious + stats.undetected + stats.harmless;

                info!(
                    "VT scan complete: {} malicious, {} suspicious, {} undetected, {} harmless",
                    stats.malicious, stats.suspicious, stats.undetected, stats.harmless
                );

                return Ok(ScanResult {
                    is_safe: stats.malicious == 0 && stats.suspicious == 0,
                    malicious_count: stats.malicious,
                    suspicious_count: stats.suspicious,
                    total_scans: total,
                });
            }

            debug!("Analysis status: {}", status);
            sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }

        Err(anyhow!(
            "VT analysis timeout after {} attempts",
            MAX_ATTEMPTS
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vt_client_creation() {
        let client = VirusTotalClient::new("test_key".to_string(), true);
        assert!(client.enabled);
        assert_eq!(client.api_key, "test_key");
    }

    #[test]
    fn test_vt_disabled() {
        let client = VirusTotalClient::new("".to_string(), false);
        assert!(!client.enabled);
    }
}
