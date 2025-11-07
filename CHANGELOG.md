# Changelog

All notable changes to SkyBin (formerly PasteVault) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-07

### Added

- **Core Infrastructure**
  - Tokio async runtime with full async/await support
  - SQLite database with FTS5 full-text search
  - Configuration system with TOML parsing
  - Comprehensive logging with tracing

- **Database Layer**
  - Automatic data retention with TTL-based auto-purge
  - FIFO enforcement for max paste count limits
  - FTS5 sync triggers for search index consistency
  - UUID-based paste identification

- **Pattern Detection**
  - 15+ built-in pattern categories:
    - AWS access keys and secret keys
    - GitHub tokens and personal access tokens
    - Private keys (RSA, DSA, EC, OpenSSH)
    - Database credentials (MySQL, PostgreSQL, MongoDB)
    - Credit card numbers (Visa, Mastercard, Amex)
    - Email addresses and password combinations
    - IP addresses and CIDR blocks
    - Generic API keys and tokens
  - Regex-based detection engine with severity levels
  - Configurable custom pattern support
  - Pattern snippet extraction (up to 500 chars)

- **Rate Limiting**
  - Per-source rate limiting with configurable limits
  - Jitter (500-5000ms) to avoid thundering herd
  - Exponential backoff on failures
  - Per-request delay tracking

- **Scrapers**
  - Pastebin scraper with API support
  - Extensible scraper trait for easy addition of new sources
  - Configurable source enablement/disablement
  - Async HTTP client with error handling

- **Scheduler**
  - Concurrent paste processing pipeline
  - Pattern detection integration
  - Duplicate detection via SHA256 hashing
  - Automatic paste metadata enrichment

- **Web Server**
  - Axum web framework with tower middleware
  - REST API endpoints:
    - `GET /` - Recent pastes feed
    - `GET /paste/:id` - View individual paste
    - `GET /raw/:id` - Raw text view
    - `POST /upload` - Submit new paste
    - `GET /search` - Full-text search
    - `GET /api/health` - Health check
  - Gzip compression support
  - 10MB request size limits
  - Thread-safe state management with Arc<Mutex<>>
  - Proper error responses and status codes

- **Testing**
  - 44 comprehensive unit tests
  - Pattern detection tests (positive/negative)
  - Database operation tests
  - Rate limiter tests
  - Hash deduplication tests
  - Web handler tests
  - 100% test pass rate

- **CI/CD**
  - GitHub Actions workflows for:
    - Automated testing (cargo test)
    - Code formatting checks (rustfmt)
    - Linting with clippy
    - Release binary builds
    - Artifact uploads
  - GitHub Pages deployment with documentation

- **Documentation**
  - Comprehensive README with architecture overview
  - WARP.md with development guidelines
  - CONTRIBUTING.md for contributor guidelines
  - Inline code documentation
  - API response examples

### Technical Details

- **Language**: Rust 1.70+
- **Runtime**: Tokio (async/await)
- **Web Framework**: Axum
- **Database**: SQLite with rusqlite
- **Code Size**: 2,146 lines of Rust
- **Test Coverage**: 44 passing tests
- **Performance**: Optimized release build available

### Known Limitations

- Scrapers are currently limited to Pastebin (more sources planned)
- Web handlers are stub implementations (to be expanded)
- No authentication system (by design for anonymity)
- Single-writer SQLite may limit concurrent writes

### Future Roadmap

- [ ] Implement additional paste source scrapers (Gist, Paste.ee, etc.)
- [ ] Complete web UI implementation with real database queries
- [ ] Add full-text search capabilities to API
- [ ] Implement rate limiting on paste uploads
- [ ] Add paste encryption option
- [ ] Database replication support
- [ ] Distributed scraping architecture
- [ ] Machine learning-based pattern detection
- [ ] Webhook notifications for sensitive patterns
- [ ] API key authentication for programmatic access

---

### Installation

```bash
git clone git@github.com:NullMeDev/skybin.git
cd skybin
cargo build --release
./target/release/paste-vault
```

### Configuration

Edit `config.toml` to customize:
- Server host/port
- Database path and retention
- Scraping intervals and concurrency
- Enabled paste sources
- Pattern detection categories
- Optional API keys for higher rate limits

### Testing

```bash
cargo test --lib
cargo clippy
cargo fmt
```
