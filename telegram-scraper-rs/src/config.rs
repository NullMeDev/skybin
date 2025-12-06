use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub skybin: SkybinConfig,
    pub scraper: ScraperConfig,
    #[serde(default)]
    pub channels: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub api_hash: String,
    pub phone: String,
    pub session_file: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkybinConfig {
    pub api_url: String,
    #[serde(default = "default_check_duplicates")]
    pub check_duplicates: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScraperConfig {
    #[serde(default = "default_stats_port")]
    pub stats_port: u16,
    #[serde(default = "default_max_file_size")]
    pub max_file_size_mb: u64,
    #[serde(default = "default_max_archive_size")]
    pub max_archive_size_mb: u64,
    #[serde(default = "default_rate_limit_delay")]
    pub rate_limit_delay_ms: u64,
    #[serde(default = "default_backoff_max")]
    pub backoff_max_seconds: u64,
}

fn default_stats_port() -> u16 { 9877 }
fn default_max_file_size() -> u64 { 5 }
fn default_max_archive_size() -> u64 { 100 }
fn default_rate_limit_delay() -> u64 { 500 }
fn default_backoff_max() -> u64 { 300 }
fn default_check_duplicates() -> bool { true }

impl Default for ScraperConfig {
    fn default() -> Self {
        Self {
            stats_port: default_stats_port(),
            max_file_size_mb: default_max_file_size(),
            max_archive_size_mb: default_max_archive_size(),
            rate_limit_delay_ms: default_rate_limit_delay(),
            backoff_max_seconds: default_backoff_max(),
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        // Security: Canonicalize path to prevent traversal attacks
        let canonical_path = std::fs::canonicalize(path)
            .map_err(|e| anyhow::anyhow!("Invalid config path: {}", e))?;
        
        // Security: Ensure config file has .toml extension
        if canonical_path.extension().and_then(|s| s.to_str()) != Some("toml") {
            return Err(anyhow::anyhow!("Config file must have .toml extension"));
        }
        
        let content = std::fs::read_to_string(canonical_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

/// Generate example config file
pub fn example_config() -> &'static str {
    r#"[telegram]
api_id = 12345678
api_hash = "your_api_hash_here"
phone = "+1234567890"
session_file = "telegram.session"

[skybin]
api_url = "http://127.0.0.1:3000"
check_duplicates = true

[scraper]
stats_port = 9877
max_file_size_mb = 5
max_archive_size_mb = 100
rate_limit_delay_ms = 500
backoff_max_seconds = 300

# Channels to monitor (usernames or invite hashes)
channels = [
    "leaboratory",
    "securitydatabase",
]
"#
}
