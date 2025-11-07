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

# Run specific test
cargo test test_name

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
Scrapers → Pattern Detection → Deduplication → SQLite (FTS5) → Web Interface
```

1. **Scrapers** (`src/scrapers/`): Async scrapers for each source (Pastebin, Gists, Paste.ee, etc.)
   - Implement the `Scraper` trait
   - Use rate limiting with jitter and exponential backoff
   - Return `DiscoveredPaste` structs

2. **Pattern Detection** (`src/patterns/`): Regex-based detection engine
   - Configured via `config.toml` `[patterns]` section
   - Detects API keys, credentials, private keys, credit cards, IPs/CIDRs
   - Tags pastes with `PatternMatch` structs and sets `is_sensitive` flag

3. **Deduplication** (`src/hash.rs`): SHA256 content hashing
   - Prevents storing duplicate pastes from different sources
   - Uses `content_hash` field with UNIQUE constraint

4. **Storage** (`src/db.rs`): SQLite with FTS5 full-text search
   - Auto-purge via trigger: deletes pastes older than `retention_days` (default: 7)
   - FIFO enforcement: caps at `max_pastes` (default: 10,000)
   - FTS5 triggers keep search index in sync automatically

5. **Web Interface** (`src/web/`): Axum web server with Askama templates
   - Routes: `/` (feed), `/paste/{id}`, `/raw/{id}`, `/search`, `/upload`
   - HTMX for dynamic updates without heavy JavaScript
   - API endpoints at `/api/pastes` and `/api/paste`

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

**Scheduler** (`src/scheduler.rs`):
- Orchestrates concurrent scraper execution
- Manages scrape intervals and retry logic
- Uses Tokio for async execution

**Rate Limiter** (`src/rate_limiter.rs`):
- Per-source rate limiting using the `governor` crate
- Adds random jitter (500-5000ms default) between requests
- Exponential backoff on failures

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

Current state (v0.1.0 - early development):
- `src/main.rs`: Entry point (currently placeholder)
- `src/models.rs`: Core data structures (implemented)

Planned modules (from README):
- `src/config.rs`: Configuration parser (TOML)
- `src/db.rs`: Database operations and queries
- `src/hash.rs`: SHA256 content hashing
- `src/patterns/`: Pattern detection engine
  - `mod.rs`, `detector.rs`, `rules.rs`
- `src/scrapers/`: Individual source scrapers
  - `mod.rs`, `traits.rs`, `pastebin.rs`, `gists.rs`, etc.
- `src/scheduler.rs`: Scraping orchestration
- `src/rate_limiter.rs`: Rate limiting logic
- `src/web/`: Web interface
  - `mod.rs`, `routes.rs`, `handlers.rs`, `templates/`

## Development Notes

### Adding New Paste Sources

1. Create new scraper in `src/scrapers/{source}.rs`
2. Implement the `Scraper` trait with `name()` and `fetch_recent()` methods
3. Return `Vec<DiscoveredPaste>` from scraper
4. Add source toggle to `config.toml` `[sources]` section
5. Register scraper in `src/scrapers/mod.rs`

### Pattern Detection

- Patterns are defined in `src/patterns/rules.rs`
- Each pattern has: name, regex, severity (low/moderate/high/critical)
- Custom patterns can be added via `config.toml` `[[patterns.custom]]` sections
- Matched patterns are stored as JSON in the `matched_patterns` field

### Deduplication Strategy

- SHA256 hash computed from paste content (normalized)
- Hash stored in `content_hash` field with UNIQUE constraint
- Duplicate inserts will fail silently (or log warning)
- Same content from different sources = one stored paste

### Testing Strategy

Tests should cover:
- Pattern detection accuracy (positive/negative cases)
- Content hash collision handling
- Rate limiter behavior
- Database triggers (auto-purge, FIFO enforcement)
- FTS5 search relevance
- Scraper error handling and retries

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
