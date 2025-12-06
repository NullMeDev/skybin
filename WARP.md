# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

**SkyBin** is a Rust-based paste aggregator that monitors public paste sites and Telegram channels for leaked credentials, API keys, and sensitive data. It scrapes 24+ paste sources, applies pattern detection, and stores results in SQLite with FTS5 full-text search.

Live instances: https://skybin.lol | https://bin.nullme.lol

## Development Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build (with LTO)

# Run
./target/release/skybin        # Requires config.toml in working directory

# Test
cargo test --lib --verbose     # Unit tests only
cargo test --doc               # Doc tests
cargo test                     # All tests (includes integration tests in tests/)

# Run specific test
cargo test test_name           # Run tests matching "test_name"
cargo test patterns::          # Run all pattern module tests

# Lint & Format
cargo fmt -- --check           # Check formatting
cargo fmt                      # Apply formatting
cargo clippy --lib -- -D warnings  # Lint with warnings as errors
```

## Architecture

### Core Components

| Module | Purpose |
|--------|---------|
| `src/scrapers/` | 24+ scrapers implementing `Scraper` trait |
| `src/patterns/` | Credential/secret detection via regex patterns |
| `src/scheduler.rs` | Paste processing pipeline with credential filtering |
| `src/anonymization.rs` | Strips PII (authors, URLs, emails) before storage |
| `src/db.rs` | SQLite with FTS5, triggers for auto-purge and dedup |
| `src/web/` | Axum web server + REST API |
| `telegram-scraper/` | Separate Python service for Telegram monitoring |

### Data Flow

```
Scraper.fetch_recent()
    ↓
DiscoveredPaste { source, source_id, content, title?, author?, url?, syntax? }
    ↓
Scheduler.process_paste()
    ├── has_credentials() filter (rejects non-credential content)
    ├── anonymize_discovered_paste() strips author, URL, sanitizes title
    ├── compute_hash_normalized() for deduplication
    ├── PatternDetector.detect() finds matches
    └── auto_title::generate_title() if title missing
    ↓
Paste { id, content_hash, matched_patterns, is_sensitive, ... }
    ↓
Database.insert_paste() (dedup via UNIQUE content_hash)
```

### Credential Filter Logic (`scheduler.rs::has_credentials`)

Content is accepted if ANY of these conditions are met:
- Contains `-----BEGIN...PRIVATE KEY-----`
- Matches credential patterns (API keys, tokens) via `credential_filter::contains_credentials()`
- Contains 1+ email:password combo (`user@domain:password`)
- Contains 1+ URL:login:pass format (stealer logs)
- Contains 3+ leak keywords (leak, dump, combo, breach, etc.)

### Database Schema

Key tables in `src/db.rs::init_schema()`:
- `pastes` - Main storage with `content_hash` UNIQUE constraint
- `pastes_fts` - FTS5 virtual table synced via triggers
- `comments` - Anonymous comments on pastes
- `scraper_stats` - Source health tracking
- `activity_logs` - Capped at 10k entries

Auto-cleanup triggers: expired pastes purged on INSERT, max 10k pastes enforced (FIFO).

## Adding a New Scraper

1. Create `src/scrapers/newsource.rs`:
```rust
use crate::models::DiscoveredPaste;
use crate::scrapers::{Scraper, ScraperResult};
use async_trait::async_trait;

pub struct NewSourceScraper;

impl NewSourceScraper {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Scraper for NewSourceScraper {
    fn name(&self) -> &str { "newsource" }
    
    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Fetch and parse pastes
        Ok(vec![])
    }
}
```

2. Register in `src/scrapers/mod.rs`:
```rust
pub mod newsource;
pub use newsource::NewSourceScraper;
```

3. Add config toggle in `src/config.rs` (SourcesConfig struct):
```rust
#[serde(default)]
pub newsource: bool,
```

4. Spawn in `src/main.rs`:
```rust
if config.sources.newsource {
    spawn_scraper("newsource", Box::new(NewSourceScraper::new()));
}
```

5. Add to `config.toml`:
```toml
[sources]
newsource = true
```

## Key APIs

### Public Endpoints
- `GET /api/pastes` - List recent pastes
- `GET /api/paste/:id` - Get paste details
- `GET /api/search?q=` - Full-text search
- `GET /api/stats` - Statistics
- `POST /api/paste` - Create paste
- `POST /api/submit-url` - Submit URL for scraping

### Admin Endpoints (`/api/x/*`)
Protected by `AdminAuth` (password in `config.toml`). Access panel at `/x`.
- `POST /api/x/login` - Get session token
- `GET /api/x/stats` - Admin statistics
- `DELETE /api/x/paste/:id` - Delete paste
- `DELETE /api/x/source/:name` - Purge all from source

## Testing

- **Unit tests**: Inline `#[cfg(test)]` modules in each source file
- **Integration tests**: `tests/e2e_scrapers_anonymization.rs`
- **In-memory DB**: Tests use `Database::open(":memory:")`

Pattern detection tests are in `src/patterns/detector.rs` and `src/patterns/rules.rs`.

## Telegram Scraper

Separate Python service in `telegram-scraper/`:
```bash
cd telegram-scraper
pip install -r requirements.txt  # telethon, aiohttp, python-dotenv
cp .env.example .env             # Set TELEGRAM_API_ID, TELEGRAM_API_HASH
python scraper.py
```

Posts discovered leaks to `POST /api/paste` with source="telegram".

## Configuration

Key `config.toml` sections:
- `[server]` - host, port, max_paste_size
- `[storage]` - db_path, retention_days (7), max_pastes (10000)
- `[scraping]` - interval_seconds (30), jitter, proxy
- `[sources]` - Boolean toggles for each scraper
- `[apis]` - pastebin_api_key, github_token (optional)
- `[patterns]` - Toggle detection categories + custom patterns array
- `[admin]` - password (set to enable /x panel)

## Important Conventions

- Scrapers return `DiscoveredPaste`, never store directly
- All pastes are anonymized before storage (no author/URL retained)
- Content deduplication via SHA256 hash of normalized content
- Pattern severities: critical > high > moderate > low
- Web UI uses Askama templates in `templates/`
- Static assets served from `static/`
