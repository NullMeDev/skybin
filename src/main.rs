use paste_vault::config::Config;
use paste_vault::db::Database;
use paste_vault::patterns::PatternDetector;
use paste_vault::rate_limiter::SourceRateLimiter;
use paste_vault::scheduler::Scheduler;
use paste_vault::scrapers::traits::Scraper;
use paste_vault::scrapers::{
    CodepadScraper, ControlcScraper, DPasteScraper, DefuseScraper, DpasteOrgScraper,
    ExternalUrlScraper, GhostbinScraper, GitHubGistsScraper, HastebinScraper, IxioScraper,
    JustPasteScraper, PasteEeScraper, PastebinScraper, PastecodeScraper, RentryScraper,
    SlexyScraper, UbuntuPastebinScraper,
};
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

    // Create external URL scraper (shared with API)
    let external_scraper = Arc::new(ExternalUrlScraper::new());
    let external_scraper_clone = external_scraper.clone();

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
    if config.sources.rentry {
        spawn_scraper("rentry", Box::new(RentryScraper::new()));
    }
    if config.sources.hastebin {
        spawn_scraper("hastebin", Box::new(HastebinScraper::new()));
    }
    if config.sources.slexy {
        spawn_scraper("slexy", Box::new(SlexyScraper::new()));
    }
    if config.sources.ubuntu_pastebin {
        spawn_scraper("ubuntu_pastebin", Box::new(UbuntuPastebinScraper::new()));
    }
    if config.sources.ghostbin {
        spawn_scraper("ghostbin", Box::new(GhostbinScraper::new()));
    }
    if config.sources.ixio {
        spawn_scraper("ixio", Box::new(IxioScraper::new()));
    }
    if config.sources.justpaste {
        spawn_scraper("justpaste", Box::new(JustPasteScraper::new()));
    }
    if config.sources.controlc {
        spawn_scraper("controlc", Box::new(ControlcScraper::new()));
    }
    if config.sources.pastecode {
        spawn_scraper("pastecode", Box::new(PastecodeScraper::new()));
    }
    if config.sources.dpaste_org {
        spawn_scraper("dpaste_org", Box::new(DpasteOrgScraper::new()));
    }
    if config.sources.defuse {
        spawn_scraper("defuse", Box::new(DefuseScraper::new()));
    }
    if config.sources.codepad {
        spawn_scraper("codepad", Box::new(CodepadScraper::new()));
    }

    // External URL scraper (always enabled for URL submissions)
    let scraper_for_task = (*external_scraper_clone).clone();
    spawn_scraper("external_url", Box::new(scraper_for_task));

    println!("✓ Scraper tasks spawned (enabled sources + external_url)");

    // Create web server state
    let app_state = AppState {
        db: db.clone(),
        url_scraper: Some(external_scraper),
    };
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
