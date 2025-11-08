use paste_vault::config::Config;
use paste_vault::db::Database;
use paste_vault::patterns::PatternDetector;
use paste_vault::rate_limiter::SourceRateLimiter;
use paste_vault::scheduler::Scheduler;
use paste_vault::scrapers::traits::Scraper;
use paste_vault::scrapers::{DPasteScraper, GitHubGistsScraper, PasteEeScraper, PastebinScraper};
use paste_vault::web::{create_router, AppState};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::from_file("config.toml")?;
    println!("✓ Configuration loaded");

    // Initialize database
    let mut db = Database::open(&config.storage.db_path)?;
    db.init_schema()?;
    println!("✓ Database initialized at {}", config.storage.db_path);
    let db = Arc::new(Mutex::new(db));

    // Create rate limiter
    let rate_limiter =
        SourceRateLimiter::new(config.scraping.jitter_min_ms, config.scraping.jitter_max_ms);
    println!("✓ Rate limiter configured");

    // Create pattern detector from config
    let detector = PatternDetector::load_from_config(&config);
    println!(
        "✓ Pattern detector initialized with {} patterns",
        detector.pattern_count()
    );
    if detector.pattern_count() == 0 {
        tracing::warn!("⚠️  No patterns enabled in config!");
    }

    let detector_clone = detector.clone();

    // Helper to spawn a scraper task
    let spawn_scraper = |name: &'static str, scraper: Box<dyn Scraper + Send + Sync>| {
        let scraper_config = config.clone();
        let detector = detector_clone.clone();
        let rate_limiter = rate_limiter.clone();
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            loop {
                match scraper.fetch_recent(&client).await {
                    Ok(discovered_pastes) => {
                        if !discovered_pastes.is_empty() {
                            println!("✓ [{}] Fetched {} pastes", name, discovered_pastes.len());
                        }
                        let mut scheduler = Scheduler::new(
                            Database::open(&scraper_config.storage.db_path).unwrap(),
                            detector.clone(),
                            rate_limiter.clone(),
                            scraper_config.scraping.interval_seconds,
                        );
                        for paste in discovered_pastes {
                            if let Err(e) = scheduler.process_paste(paste) {
                                tracing::warn!("[{}] Failed to process paste: {}", name, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("[{}] Scraper error: {}", name, e);
                    }
                }
                tokio::time::sleep(Duration::from_secs(
                    scraper_config.scraping.interval_seconds,
                ))
                .await;
            }
        });
    };

    // Spawn enabled scrapers
    if config.sources.pastebin {
        spawn_scraper("pastebin", Box::new(PastebinScraper::new()));
    }
    if config.sources.gists {
        let gists_scraper = if !config.apis.github_token.is_empty() {
            GitHubGistsScraper::with_token(config.apis.github_token.clone())
        } else {
            GitHubGistsScraper::new()
        };
        spawn_scraper("gists", Box::new(gists_scraper));
    }
    if config.sources.paste_ee {
        spawn_scraper("paste_ee", Box::new(PasteEeScraper::new()));
    }
    if config.sources.dpaste {
        spawn_scraper("dpaste", Box::new(DPasteScraper::new()));
    }

    println!("✓ Scraper tasks spawned (enabled sources)");

    // Create web server state
    let app_state = AppState { db: db.clone() };
    println!("✓ Web server state created");

    // Create router
    let app = create_router(app_state);
    println!("✓ Router configured");

    // Create listener
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!(
        "\n==> PasteVault v{} initialized successfully!",
        env!("CARGO_PKG_VERSION")
    );
    println!("    Server listening on http://{}", addr);
    println!("    Data retention: {} days", config.storage.retention_days);
    println!(
        "    Scrape interval: {} seconds",
        config.scraping.interval_seconds
    );
    println!("\n    Press Ctrl+C to stop the server\n");

    // Start server
    axum::serve(listener, app).await?;

    Ok(())
}
