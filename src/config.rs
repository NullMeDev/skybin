use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    TomlError(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

/// Main configuration struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub scraping: ScrapingConfig,
    pub sources: SourcesConfig,
    pub apis: ApisConfig,
    pub patterns: PatternsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_paste_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub db_path: String,
    pub retention_days: i64,
    pub max_pastes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapingConfig {
    pub interval_seconds: u64,
    pub concurrent_scrapers: usize,
    pub jitter_min_ms: u64,
    pub jitter_max_ms: u64,
    pub retries: usize,
    pub backoff_ms: u64,
    pub proxy: String,
    pub user_agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    pub pastebin: bool,
    pub gists: bool,
    pub paste_ee: bool,
    pub rentry: bool,
    pub ghostbin: bool,
    pub slexy: bool,
    pub dpaste: bool,
    pub hastebin: bool,
    pub ubuntu_pastebin: bool,
}

impl SourcesConfig {
    /// Get list of enabled sources
    pub fn enabled_sources(&self) -> Vec<&str> {
        let mut sources = Vec::new();
        if self.pastebin {
            sources.push("pastebin");
        }
        if self.gists {
            sources.push("gists");
        }
        if self.paste_ee {
            sources.push("paste_ee");
        }
        if self.rentry {
            sources.push("rentry");
        }
        if self.ghostbin {
            sources.push("ghostbin");
        }
        if self.slexy {
            sources.push("slexy");
        }
        if self.dpaste {
            sources.push("dpaste");
        }
        if self.hastebin {
            sources.push("hastebin");
        }
        if self.ubuntu_pastebin {
            sources.push("ubuntu_pastebin");
        }
        sources
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApisConfig {
    pub pastebin_api_key: String,
    pub github_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternsConfig {
    pub aws_keys: bool,
    pub credit_cards: bool,
    pub emails: bool,
    pub email_password_combos: bool,
    pub ip_cidr: bool,
    pub private_keys: bool,
    pub db_credentials: bool,
    pub generic_api_keys: bool,
    #[serde(default)]
    pub custom: Vec<CustomPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPattern {
    pub name: String,
    pub regex: String,
    pub severity: String,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from a TOML string
    pub fn from_str(content: &str) -> Result<Self> {
        let config = toml::from_str(content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let toml_str = r#"
[server]
host = "127.0.0.1"
port = 3000
max_paste_size = 500000

[storage]
db_path = "test.db"
retention_days = 7
max_pastes = 5000

[scraping]
interval_seconds = 300
concurrent_scrapers = 3
jitter_min_ms = 500
jitter_max_ms = 5000
retries = 3
backoff_ms = 500
proxy = ""
user_agents = ["Mozilla/5.0"]

[sources]
pastebin = true
gists = false
paste_ee = true
rentry = false
ghostbin = false
slexy = true
dpaste = true
hastebin = false
ubuntu_pastebin = false

[apis]
pastebin_api_key = ""
github_token = ""

[patterns]
aws_keys = true
credit_cards = true
emails = true
email_password_combos = true
ip_cidr = true
private_keys = true
db_credentials = true
generic_api_keys = true
"#;

        let config = Config::from_str(toml_str).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.storage.db_path, "test.db");
        assert!(config.sources.pastebin);
        assert!(!config.sources.gists);
    }

    #[test]
    fn test_enabled_sources() {
        let toml_str = r#"
[server]
host = "0.0.0.0"
port = 8080
max_paste_size = 1000000

[storage]
db_path = "test.db"
retention_days = 7
max_pastes = 10000

[scraping]
interval_seconds = 300
concurrent_scrapers = 5
jitter_min_ms = 500
jitter_max_ms = 5000
retries = 3
backoff_ms = 500
proxy = ""
user_agents = ["Mozilla/5.0"]

[sources]
pastebin = true
gists = true
paste_ee = false
rentry = false
ghostbin = false
slexy = true
dpaste = false
hastebin = false
ubuntu_pastebin = false

[apis]
pastebin_api_key = ""
github_token = ""

[patterns]
aws_keys = true
credit_cards = true
emails = true
email_password_combos = true
ip_cidr = true
private_keys = true
db_credentials = true
generic_api_keys = true
"#;

        let config = Config::from_str(toml_str).unwrap();
        let enabled = config.sources.enabled_sources();
        assert_eq!(enabled.len(), 3);
        assert!(enabled.contains(&"pastebin"));
        assert!(enabled.contains(&"gists"));
        assert!(enabled.contains(&"slexy"));
    }
}
