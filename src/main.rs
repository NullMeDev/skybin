use paste_vault::admin::AdminAuth;
use paste_vault::config::Config;
use paste_vault::db::Database;
use paste_vault::patterns::PatternDetector;
use paste_vault::rate_limiter::{ApiRateLimiters, SourceRateLimiter};
use paste_vault::scheduler::Scheduler;
use paste_vault::scrapers::traits::Scraper;
use paste_vault::scrapers::{
    BpastScraper, BpasteScraper, CodepadScraper, ControlcScraper, DPasteScraper, DefuseScraper,
    DpasteOrgScraper, ExternalUrlScraper, GhostbinScraper, GitHubCodeScraper, GitHubGistsScraper,
    HastebinScraper, IdeoneScraper, IxioScraper, JustPasteScraper, Paste2Scraper, PasteEeScraper,
    PasteRsScraper, PastebinPlScraper, PastebinScraper, PastecodeScraper, PastesioScraper,
    PsbdmpScraper, QuickpasteScraper, RentryScraper, SlexyScraper, SprungeScraper, TermbinScraper,
    TorPastesScraper, UbuntuPastebinScraper,
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

    // Helper to spawn a scraper task with health tracking and exponential backoff recovery
    let spawn_scraper = |name: &'static str, scraper: Box<dyn Scraper + Send + Sync>| {
        let scraper_config = config.clone();
        let detector = detector_clone.clone();
        let rate_limiter = rate_limiter.clone();
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let base_interval = scraper_config.scraping.interval_seconds;
            let max_backoff = 3600u64; // Max 1 hour backoff
            let mut consecutive_failures = 0u32;
            
            loop {
                let result = scraper.fetch_recent(&client).await;
                
                // Log scraper health stats
                let (success, pastes_found) = match &result {
                    Ok(pastes) => (true, pastes.len()),
                    Err(_) => (false, 0),
                };
                
                if let Ok(mut stats_db) = Database::open(&scraper_config.storage.db_path) {
                    let _ = stats_db.log_scraper_stat(name, success, pastes_found);
                }
                
                // Calculate next sleep interval with exponential backoff on failure
                let sleep_interval = match result {
                    Ok(discovered_pastes) => {
                        // Reset backoff on success
                        if consecutive_failures > 0 {
                            println!("✓ [{}] Recovered after {} failures", name, consecutive_failures);
                        }
                        consecutive_failures = 0;
                        
                        if !discovered_pastes.is_empty() {
                            println!("✓ [{}] Fetched {} pastes", name, discovered_pastes.len());
                        }
                        let mut scheduler = Scheduler::new(
                            Database::open(&scraper_config.storage.db_path).unwrap(),
                            detector.clone(),
                            rate_limiter.clone(),
                            base_interval,
                        );
                        for paste in discovered_pastes {
                            if let Err(e) = scheduler.process_paste(paste) {
                                tracing::warn!("[{}] Failed to process paste: {}", name, e);
                            }
                        }
                        base_interval
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        
                        // Exponential backoff: base * 2^failures, capped at max_backoff
                        let backoff = (base_interval * 2u64.saturating_pow(consecutive_failures.min(10)))
                            .min(max_backoff);
                        
                        if consecutive_failures == 1 {
                            tracing::warn!("[{}] Scraper error: {} (retry in {}s)", name, e, backoff);
                        } else {
                            tracing::warn!(
                                "[{}] Scraper error (failure #{}, backing off {}s): {}",
                                name, consecutive_failures, backoff, e
                            );
                        }
                        backoff
                    }
                };
                
                tokio::time::sleep(Duration::from_secs(sleep_interval)).await;
            }
        });
    };

    // Spawn enabled scrapers
    if config.sources.pastebin {
        spawn_scraper("pastebin", Box::new(PastebinScraper::new()));
    }
    // GitHub Code Search (recommended - finds exposed secrets in repos)
    if config.sources.github {
        let github_scraper = if !config.apis.github_token.is_empty() {
            GitHubCodeScraper::with_token(config.apis.github_token.clone())
        } else {
            GitHubCodeScraper::new()
        };
        spawn_scraper("github", Box::new(github_scraper));
    }
    // Legacy gists scraper (deprecated, use github instead)
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
    if config.sources.ideone {
        spawn_scraper("ideone", Box::new(IdeoneScraper::new()));
    }
    if config.sources.bpaste {
        spawn_scraper("bpaste", Box::new(BpasteScraper::new()));
    }
    if config.sources.termbin {
        spawn_scraper("termbin", Box::new(TermbinScraper::new()));
    }
    if config.sources.sprunge {
        spawn_scraper("sprunge", Box::new(SprungeScraper::new()));
    }
    if config.sources.paste_rs {
        spawn_scraper("paste_rs", Box::new(PasteRsScraper::new()));
    }
    if config.sources.paste2 {
        spawn_scraper("paste2", Box::new(Paste2Scraper::new()));
    }
    if config.sources.pastebin_pl {
        spawn_scraper("pastebin_pl", Box::new(PastebinPlScraper::new()));
    }
    if config.sources.quickpaste {
        spawn_scraper("quickpaste", Box::new(QuickpasteScraper::new()));
    }
    if config.sources.psbdmp {
        spawn_scraper("psbdmp", Box::new(PsbdmpScraper::new()));
    }
    if config.sources.tor_pastes {
        let tor_scraper = TorPastesScraper::with_proxy(config.scraping.proxy.clone());
        spawn_scraper("tor_pastes", Box::new(tor_scraper));
    }
    if config.sources.pastesio {
        spawn_scraper("pastesio", Box::new(PastesioScraper::new()));
    }
    if config.sources.bpast {
        spawn_scraper("bpast", Box::new(BpastScraper::new()));
    }

    // External URL scraper
    let scraper_for_task = (*external_scraper_clone).clone();
    spawn_scraper("external_url", Box::new(scraper_for_task));

    println!("✓ Scraper tasks spawned (enabled sources + external_url)");

    // Initialize admin auth if password configured
    let admin = if !config.admin.password.is_empty() && !config.admin.password.starts_with("{{") {
        println!("✓ Admin panel enabled at /x");
        Some(Arc::new(AdminAuth::new(&config.admin.password)))
    } else {
        println!("⚠️  Admin panel disabled (no password configured)");
        None
    };

    // Initialize API rate limiters
    let rate_limiters = Arc::new(ApiRateLimiters::new());
    println!("✓ API rate limiters initialized");

    // Create web server state
    let app_state = AppState {
        db: db.clone(),
        url_scraper: Some(external_scraper),
        admin,
        rate_limiters,
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
