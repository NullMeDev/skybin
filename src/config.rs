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
    #[serde(default)]
    pub admin: AdminConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdminConfig {
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_paste_size: usize,
    #[serde(default)]
    pub max_upload_size: Option<usize>,
    #[serde(default)]
    pub enable_virustotal_scan: Option<bool>,
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
    #[serde(default)]
    pub github: bool, // GitHub code search for exposed secrets
    pub gists: bool, // Legacy gists scraper (deprecated)
    pub paste_ee: bool,
    pub rentry: bool,
    pub ghostbin: bool,
    pub slexy: bool,
    pub dpaste: bool,
    pub hastebin: bool,
    pub ubuntu_pastebin: bool,
    #[serde(default)]
    pub ixio: bool,
    #[serde(default)]
    pub justpaste: bool,
    #[serde(default)]
    pub controlc: bool,
    #[serde(default)]
    pub pastecode: bool,
    #[serde(default)]
    pub dpaste_org: bool,
    #[serde(default)]
    pub defuse: bool,
    #[serde(default)]
    pub codepad: bool,
    #[serde(default)]
    pub ideone: bool,
    #[serde(default)]
    pub bpaste: bool,
    #[serde(default)]
    pub termbin: bool,
    #[serde(default)]
    pub sprunge: bool,
    #[serde(default)]
    pub paste_rs: bool,
    #[serde(default)]
    pub paste2: bool,
    #[serde(default)]
    pub pastebin_pl: bool,
    #[serde(default)]
    pub quickpaste: bool,
    #[serde(default)]
    pub psbdmp: bool,
    #[serde(default)]
    pub tor_pastes: bool,
    #[serde(default)]
    pub pastesio: bool,
    #[serde(default)]
    pub bpast: bool,
    #[serde(default)]
    pub pastefs: bool,
    #[serde(default)]
    pub kbinbin: bool,
    #[serde(default)]
    pub snippet: bool,
    #[serde(default)]
    pub privatebin: bool,
    #[serde(default)]
    pub zerobin: bool,
}

impl SourcesConfig {
    /// Get list of enabled sources
    pub fn enabled_sources(&self) -> Vec<&str> {
        let mut sources = Vec::new();
        if self.pastebin {
            sources.push("pastebin");
        }
        if self.github {
            sources.push("github");
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
        if self.ixio {
            sources.push("ixio");
        }
        if self.justpaste {
            sources.push("justpaste");
        }
        if self.controlc {
            sources.push("controlc");
        }
        if self.pastecode {
            sources.push("pastecode");
        }
        if self.dpaste_org {
            sources.push("dpaste_org");
        }
        if self.defuse {
            sources.push("defuse");
        }
        if self.codepad {
            sources.push("codepad");
        }
        if self.ideone {
            sources.push("ideone");
        }
        if self.bpaste {
            sources.push("bpaste");
        }
        if self.termbin {
            sources.push("termbin");
        }
        if self.sprunge {
            sources.push("sprunge");
        }
        if self.paste_rs {
            sources.push("paste_rs");
        }
        if self.paste2 {
            sources.push("paste2");
        }
        if self.pastebin_pl {
            sources.push("pastebin_pl");
        }
        if self.quickpaste {
            sources.push("quickpaste");
        }
        if self.psbdmp {
            sources.push("psbdmp");
        }
        if self.tor_pastes {
            sources.push("tor_pastes");
        }
        if self.pastesio {
            sources.push("pastesio");
        }
        if self.bpast {
            sources.push("bpast");
        }
        if self.pastefs {
            sources.push("pastefs");
        }
        if self.kbinbin {
            sources.push("kbinbin");
        }
        if self.snippet {
            sources.push("snippet");
        }
        if self.privatebin {
            sources.push("privatebin");
        }
        if self.zerobin {
            sources.push("zerobin");
        }
        sources
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApisConfig {
    pub pastebin_api_key: String,
    pub github_token: String,
    #[serde(default)]
    pub virustotal_api_key: Option<String>,
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
    #[serde(default = "default_true")]
    pub discord_tokens: bool,
    #[serde(default = "default_true")]
    pub oauth_tokens: bool,
    #[serde(default = "default_true")]
    pub streaming_creds: bool,
    #[serde(default = "default_true")]
    pub jwt_tokens: bool,
    #[serde(default = "default_true")]
    pub payment_keys: bool,
    #[serde(default = "default_true")]
    pub cloud_tokens: bool,
    #[serde(default)]
    pub custom: Vec<CustomPattern>,
}

fn default_true() -> bool {
    true
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
        // Security: Canonicalize path to prevent traversal attacks
        let canonical_path = std::fs::canonicalize(path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Invalid config path: {}", e),
            )
        })?;

        // Security: Ensure config file has .toml extension
        if canonical_path.extension().and_then(|s| s.to_str()) != Some("toml") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Config file must have .toml extension",
            )
            .into());
        }

        let content = std::fs::read_to_string(canonical_path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from a TOML string
    pub fn from_toml_str(content: &str) -> Result<Self> {
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

        let config = Config::from_toml_str(toml_str).unwrap();
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

        let config = Config::from_toml_str(toml_str).unwrap();
        let enabled = config.sources.enabled_sources();
        assert_eq!(enabled.len(), 3);
        assert!(enabled.contains(&"pastebin"));
        assert!(enabled.contains(&"gists"));
        assert!(enabled.contains(&"slexy"));
    }
}
