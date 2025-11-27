# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Essential Commands

### Building and Running
```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run the application
cargo run

# Run with debug logging
RUST_LOG=debug cargo run

# Run with trace-level logging for specific modules
RUST_LOG=paste_vault=trace cargo run
```

### Testing and Linting
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test by name substring
cargo test hash_consistency

# Run tests in a specific module
cargo test patterns::detector

# Run integration tests
cargo test --test e2e_scrapers_anonymization

# Run linter (clippy)
cargo clippy

# Run clippy with all warnings
cargo clippy -- -W clippy::all

# Format code
cargo fmt

# Check formatting without changing files
cargo fmt -- --check
```

### Database Operations
```bash
# Database is auto-initialized on first run at pastevault.db
# To reset database, delete the files:
rm pastevault.db pastevault.db-wal pastevault.db-shm

# Manual migration (if needed)
sqlite3 pastevault.db < migrations/001_initial.sql
```

## Architecture Overview

PasteVault is a concurrent paste aggregator that scrapes multiple public paste sites for sensitive data leaks while also functioning as an anonymous pastebin.

### Data Flow
```
                                    ┌─────────────────────┐
                                    │  External URL API   │
                                    │  POST /api/submit-url│
                                    └──────────┬──────────┘
                                               │
                                               ▼
Scrapers → Anonymization → Pattern Detection → Deduplication → SQLite (FTS5) → Web Interface
                                                                                    ↓
                                                                          Language Detection
```

1. **Scrapers** (`src/scrapers/`): Async scrapers for 13 sources
   - **Active scrapers** (have public listing/archive pages):
     - Pastebin (`pastebin.rs`) - scrapes archive page
     - GitHub Gists (`github_gists.rs`) - uses public API, optional token for higher rate limits
     - Slexy (`slexy.rs`) - scrapes /recent page
     - ControlC (`controlc.rs`) - scrapes /recent page
   - **Placeholder scrapers** (no public recent API - use external URL submission):
     - Paste.ee, DPaste, Rentry, Hastebin, Ubuntu Pastebin, ix.io, JustPaste.it, Ghostbin (defunct)
   - **External URL scraper**: Always enabled, processes URLs submitted via API
   - All scrapers implement the `Scraper` trait
   - Use rate limiting with jitter and exponential backoff
   - Return `Vec<DiscoveredPaste>`

2. **Anonymization** (`src/anonymization.rs`): Privacy-first processing
   - Strips author names, URLs, emails from scraped pastes
   - Sanitizes titles to remove PII
   - Removes emojis and potentially identifying information
   - User-submitted pastes are fully anonymous (no IP logging)

3. **Pattern Detection** (`src/patterns/`): Regex-based detection engine
   - Configured via `config.toml` `[patterns]` section
   - Detects API keys, credentials, private keys, credit cards, IPs/CIDRs
   - Tags pastes with `PatternMatch` structs and sets `is_sensitive` flag
   - Supports custom user-defined patterns with severity levels

4. **Deduplication** (`src/hash.rs`): SHA256 content hashing
   - Prevents storing duplicate pastes from different sources
   - Uses `content_hash` field with UNIQUE constraint

5. **Storage** (`src/db.rs`): SQLite with FTS5 full-text search
   - Auto-purge via trigger: deletes pastes older than `retention_days` (default: 7)
   - FIFO enforcement: caps at `max_pastes` (default: 10,000)
   - FTS5 triggers keep search index in sync automatically

6. **Language Detection** (`src/lang_detect.rs`): Auto-detect syntax
   - Pattern-based detection for 15+ languages
   - Applies to both scraped and user-uploaded pastes
   - Used for syntax highlighting in web interface

7. **Web Interface** (`src/web/`): Dual API/HTML architecture
   - **HTML routes**: `/`, `/paste/{id}`, `/raw/{id}`, `/search`, `/upload`
   - **API routes**: `/api/pastes`, `/api/paste/{id}`, `/api/search`, `/api/stats`, `/api/health`, `/api/submit-url`
   - Axum web framework with Askama template rendering
   - JSON API for programmatic access

8. **External URL Submission** (`src/scrapers/external_url.rs`): User-submitted paste monitoring
   - Submit paste URLs from ANY paste site via `POST /api/submit-url`
   - Queue-based processing (up to 10 URLs per scrape cycle)
   - Automatic source detection from URL (pastebin, gist, rentry, etc.)
   - Deduplication prevents re-fetching same URLs

### Key Components

**Configuration** (`src/config.rs`):
- Parsed from `config.toml` at startup
- Controls scraping intervals, rate limits, enabled sources, pattern detection
- API keys for Pastebin PRO and GitHub tokens for higher rate limits

**Models** (`src/models.rs`):
- `Paste`: Final stored paste with detected patterns
- `DiscoveredPaste`: Intermediate format from scrapers (before storage)
- `PatternMatch`: Individual pattern detection result
- `SearchFilters`: Query parameters for search
- `Stats`: Statistics about pastes and sources

**Scheduler** (`src/scheduler.rs`):
- Orchestrates concurrent scraper execution
- Manages scrape intervals and retry logic
- Uses Tokio for async execution
- Processes discovered pastes through the pipeline

**Rate Limiter** (`src/rate_limiter.rs`):
- Per-source rate limiting using the `governor` crate
- Adds random jitter (500-5000ms default) between requests
- Exponential backoff on failures

**Anonymization** (`src/anonymization.rs`):
- Strips author names, URLs, and emails from scraped content
- Sanitizes titles to remove PII (emails, @usernames, URLs)
- Removes emojis from content and titles
- Validates anonymity before storage

## Configuration

The `config.toml` file drives all behavior:

- `[server]`: Web server host/port and paste size limits
- `[storage]`: Database path, retention days, max paste count
- `[scraping]`: Intervals, concurrency, retry logic, user agents, proxy
- `[sources]`: Enable/disable individual paste sources
- `[apis]`: Optional API keys for higher rate limits
- `[patterns]`: Toggle detection categories, add custom regex patterns

Changes to `config.toml` require a restart to take effect.

## Database Schema

Located in `migrations/001_initial.sql`:

**pastes** table:
- Primary storage with UUID primary key
- `content_hash` (SHA256) with UNIQUE constraint for deduplication
- `matched_patterns`: JSON array of detected patterns
- `is_sensitive`: Boolean flag for sensitive content
- Indexed on: `expires_at`, `content_hash`, `created_at`, `source`, `is_sensitive`

**pastes_fts** virtual table:
- FTS5 full-text search on `title` and `content`
- Kept in sync via INSERT/UPDATE/DELETE triggers

**Triggers**:
- `auto_purge_expired`: Deletes expired pastes on each insert
- `enforce_max_pastes`: FIFO deletion when max_pastes exceeded
- FTS sync triggers: Keep search index updated

**short_ids** table:
- Maps user-friendly short IDs to paste UUIDs (for future use)

## Module Structure

Current state (v0.2.0 - fully implemented):

**Core modules** (root of `src/`):
- `main.rs`: Application entry point, scraper task spawning
- `lib.rs`: Library exports for all modules
- `models.rs`: Data structures (`Paste`, `DiscoveredPaste`, `PatternMatch`, `SearchFilters`)
- `config.rs`: TOML configuration parser and validation
- `db.rs`: SQLite database layer with FTS5 search
- `hash.rs`: SHA256 content hashing for deduplication
- `scheduler.rs`: Paste processing pipeline orchestration
- `rate_limiter.rs`: Per-source rate limiting with jitter
- `anonymization.rs`: PII stripping and privacy protection
- `lang_detect.rs`: Language/syntax auto-detection

**Pattern detection** (`src/patterns/`):
- `mod.rs`: Module exports and detector instantiation
- `detector.rs`: Pattern matching engine with category toggles
- `rules.rs`: Built-in regex patterns (API keys, credentials, private keys, etc.)

**Scrapers** (`src/scrapers/`):
- `mod.rs`: Scraper registration and exports
- `traits.rs`: `Scraper` trait definition and error types
- **Active scrapers:**
  - `pastebin.rs`: Pastebin.com - scrapes archive page
  - `github_gists.rs`: GitHub Gists - public API with optional token
  - `slexy.rs`: Slexy.org - scrapes /recent page
  - `controlc.rs`: ControlC.com - scrapes /recent page
- **Placeholder scrapers** (no public recent API):
  - `paste_ee.rs`: Paste.ee
  - `dpaste.rs`: DPaste.org
  - `rentry.rs`: Rentry.co
  - `hastebin.rs`: Hastebin/Toptal
  - `ubuntu_pastebin.rs`: paste.ubuntu.com
  - `ixio.rs`: ix.io
  - `justpaste.rs`: JustPaste.it
  - `ghostbin.rs`: Ghostbin (defunct since 2021)
- `external_url.rs`: External URL queue scraper (always enabled)

**Web interface** (`src/web/`):
- `mod.rs`: Router setup, state definition (`AppState`), API response types
- `handlers.rs`: Route handlers for both HTML and JSON endpoints

**AppState** structure:
```rust
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub url_scraper: Option<Arc<ExternalUrlScraper>>,  // For /api/submit-url
}
```

**Templates** (`templates/`):
- `base.html`: Base layout template
- `dashboard.html`: Main feed page
- `paste_detail.html`: Individual paste view
- `search.html`: Search interface
- `upload.html`: Paste submission form

## Development Notes

### Adding New Paste Sources

1. Create new scraper in `src/scrapers/{source}.rs`
2. Implement the `Scraper` trait with `name()` and `fetch_recent()` methods
3. Return `Vec<DiscoveredPaste>` from scraper
4. Add source toggle to `config.toml` `[sources]` section
5. Register scraper in `src/scrapers/mod.rs` (add to exports)
6. Spawn scraper task in `src/main.rs` with conditional check

Example:
```rust
use crate::scrapers::traits::{Scraper, ScraperResult};
use async_trait::async_trait;

pub struct MySiteScraper;

impl MySiteScraper {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Scraper for MySiteScraper {
    fn name(&self) -> &str {
        "mysite"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // Fetch and parse pastes from source
        // Return Vec<DiscoveredPaste>
    }
}
```

### Pattern Detection

- Patterns are defined in `src/patterns/rules.rs`
- Each pattern has: name, regex, severity (low/moderate/high/critical)
- Custom patterns can be added via `config.toml` `[[patterns.custom]]` sections
- Matched patterns are stored as JSON in the `matched_patterns` field
- Categories: AWS keys, GitHub tokens, private keys, credit cards, email:password combos, IPs/CIDRs
- Detector loaded from config with `PatternDetector::load_from_config()`

### Deduplication Strategy

- SHA256 hash computed from paste content (normalized)
- Hash stored in `content_hash` field with UNIQUE constraint
- Duplicate inserts will fail silently (or log warning)
- Same content from different sources = one stored paste
- Hash function: `crate::hash::compute_hash_normalized()`

### Web Development

**Askama Templates**:
- Templates in `templates/` directory
- Use `#[derive(Template)]` with `#[template(path = "...html")]`
- Automatic HTML escaping for security
- Base template (`base.html`) for consistent layout

**Dual Architecture**:
- HTML routes return Askama templates (e.g., `DashboardTemplate`)
- API routes return `Json<ApiResponse<T>>` with typed responses
- Same data models serve both HTML and API endpoints
- State management via `Arc<Mutex<Database>>` for thread-safe access

**Adding New Routes**:
1. Define handler in `src/web/handlers.rs`
2. Add template struct with `#[derive(Template)]` if HTML route
3. Register route in `create_router()` in `src/web/mod.rs`
4. Use `State(state): State<AppState>` extractor for DB access
5. Access URL scraper via `state.url_scraper` for submission endpoints

### External URL Submission API

Submit paste URLs from any source for monitoring:

```bash
# Single URL
curl -X POST http://localhost:8081/api/submit-url \
  -H "Content-Type: application/json" \
  -d '{"url": "https://pastebin.com/AbCd1234", "urls": []}'

# Multiple URLs
curl -X POST http://localhost:8081/api/submit-url \
  -H "Content-Type: application/json" \
  -d '{"url": "", "urls": ["https://pastebin.com/abc", "https://gist.github.com/user/xyz"]}'
```

Supported source detection: pastebin.com, gist.github.com, paste.ee, dpaste.*, rentry.*, hastebin.*, or "external" for others.

### Testing Strategy

Tests cover:
- Pattern detection accuracy (positive/negative cases)
- Content hash consistency and collision handling
- Rate limiter behavior and jitter
- Database triggers (auto-purge, FIFO enforcement)
- FTS5 search relevance and indexing
- Scraper parsing and error handling
- Anonymization PII stripping
- Language detection accuracy
- Web API response formats

Example test commands:
```bash
# Test pattern detection
cargo test patterns::detector

# Test specific pattern category
cargo test test_aws_key_detection

# Test hash consistency
cargo test hash::test_hash_consistency

# Test database operations
cargo test db::test_insert_and_retrieve

# Test anonymization
cargo test anonymization::test_verify_anonymity
```

### Dependencies

Key crates:
- **tokio**: Async runtime for concurrent I/O
- **axum**: Web framework (by Tokio team)
- **askama**: Type-safe templates
- **rusqlite**: SQLite bindings with bundled library
- **reqwest**: HTTP client for scrapers
- **regex**: Pattern matching
- **governor**: Rate limiting
- **chrono**: Date/time handling
- **uuid**: Unique ID generation
- **sha2**: Content hashing

### Performance Considerations

- **Concurrency**: Adjust `concurrent_scrapers` based on CPU/network
- **Rate limiting**: Respect source site limits to avoid bans
- **Database**: SQLite is single-writer; high write concurrency may cause locks
- **Memory**: Each paste held in memory briefly during processing
- **FTS5 search**: Very fast for queries, but adds overhead to inserts/updates

### Security Notes

- No authentication required for read/write (by design for anonymity)
- HTML escaping applied to all user input via Askama templates
- Rate limiting on paste submission endpoint (per-IP)
- Content size limits enforced (`max_paste_size` in config)
- Auto-purge minimizes data exposure (7-day default retention)
