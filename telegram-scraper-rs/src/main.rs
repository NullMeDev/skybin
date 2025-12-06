mod archive;
mod classifier;
mod config;
mod server;
mod skybin;
mod stats;
mod telegram;

use std::env;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use stats::new_shared;
use telegram::TelegramScraper;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    info!("🚀 SkyBin Telegram Scraper v{}", VERSION);
    
    // Parse args
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-v" => {
                println!("telegram-scraper v{}", VERSION);
                return Ok(());
            }
            "--example-config" => {
                println!("{}", config::example_config());
                return Ok(());
            }
            _ => {}
        }
    }
    
    // Load config
    let config_path = args.get(1).map(|s| s.as_str()).unwrap_or("config.toml");
    
    let config = match Config::load(config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load config from {}: {}", config_path, e);
            error!("Run with --example-config to see an example configuration");
            std::process::exit(1);
        }
    };
    
    info!("📋 Loaded config from {}", config_path);
    
    // Create shared stats
    let stats = new_shared();
    
    // Start stats HTTP server
    let stats_clone = Arc::clone(&stats);
    let stats_port = config.scraper.stats_port;
    tokio::spawn(async move {
        server::start_server(stats_clone, stats_port).await;
    });
    
    // Connect to Telegram
    let (scraper, _file_rx) = match TelegramScraper::connect(config, Arc::clone(&stats)).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to connect to Telegram: {}", e);
            std::process::exit(1);
        }
    };
    
    let scraper = Arc::new(scraper);
    
    // Join configured channels
    if let Err(e) = scraper.join_channels().await {
        error!("Error joining channels: {}", e);
    }
    
    // Start real-time listener
    info!("🎯 Starting real-time message processing...");
    
    if let Err(e) = scraper.start_realtime_listener().await {
        error!("Fatal error in listener: {}", e);
        std::process::exit(1);
    }
    
    Ok(())
}

fn print_help() {
    println!(r#"
SkyBin Telegram Scraper v{}

A high-performance Telegram scraper that monitors channels for password files
in archives and posts them to SkyBin.

USAGE:
    telegram-scraper [OPTIONS] [CONFIG_FILE]

ARGUMENTS:
    CONFIG_FILE    Path to config file (default: config.toml)

OPTIONS:
    -h, --help           Print help information
    -v, --version        Print version
    --example-config     Print example configuration

FEATURES:
    - Real-time message processing (no polling)
    - Supports zip, rar, 7z, tar.gz, tar.bz2 archives
    - Nested archive extraction (up to 2 levels)
    - Pre-post deduplication
    - Exponential backoff for rate limits
    - HTTP stats endpoint for admin panel
    - Auto-join channels from SkyBin invite detection

ENVIRONMENT:
    RUST_LOG    Set log level (default: info)
                Examples: debug, info, warn, error

EXAMPLES:
    telegram-scraper                    # Use config.toml
    telegram-scraper /path/to/config    # Custom config path
    telegram-scraper --example-config   # Show example config
"#, VERSION);
}
