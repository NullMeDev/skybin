use paste_vault::config::Config;
use paste_vault::db::Database;
use paste_vault::patterns::PatternDetector;
use paste_vault::rate_limiter::SourceRateLimiter;
use paste_vault::scheduler::Scheduler;
use paste_vault::web::{create_router, AppState};
use std::sync::{Arc, Mutex};

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
    let rate_limiter = SourceRateLimiter::new(
        config.scraping.jitter_min_ms,
        config.scraping.jitter_max_ms,
    );
    println!("✓ Rate limiter configured");

    // Create pattern detector
    let patterns = vec![]; // TODO: load from config
    let detector = PatternDetector::new(patterns);
    println!("✓ Pattern detector initialized");

    // Create scheduler (note: would need Arc-wrapped DB)
    let _scheduler = Scheduler::new(
        Database::open(&config.storage.db_path)?,
        detector,
        rate_limiter,
        config.scraping.interval_seconds,
    );
    println!("✓ Scheduler created");

    // Create web server state
    let app_state = AppState {
        db: db.clone(),
    };
    println!("✓ Web server state created");

    // Create router
    let app = create_router(app_state);
    println!("✓ Router configured");

    // Create listener
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("\n✅ PasteVault v{} initialized successfully!", env!("CARGO_PKG_VERSION"));
    println!("   🌐 Server listening on http://{}", addr);
    println!("   📊 Data retention: {} days", config.storage.retention_days);
    println!("   ⏱️  Scrape interval: {} seconds", config.scraping.interval_seconds);
    println!("\n   Press Ctrl+C to stop the server\n");

    // Start server
    axum::serve(listener, app)
        .await?;

    Ok(())
}
