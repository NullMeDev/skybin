# Changelog

All notable changes to SkyBin will be documented in this file.

## [2.1.0] - 2025-12-05

### Added
- **Secret Extraction Pipeline** - comprehensive credential extraction with 35+ secret categories
- **Per-secret deduplication** - SHA256 hash of (type+value) prevents duplicate alerts
- **Categorized output files** - secrets dumped to respective files (AWS_Keys.txt, Discord_Tokens.txt, etc.)
- **Server secret exclusion** - `.excluded_secrets` file + auto-excluded environment variables
- **80+ regex patterns** - covers AWS, GCP, GitHub, GitLab, OpenAI, Discord, Stripe, JWT, and more
- **Rust `secret_extractor` module** - parallel extraction in Rust scrapers
- **Python `credential_extractor.py`** - extraction for Telegram scraper

### Technical
- New `seen_secrets` database table for deduplication tracking
- Output directory: `/opt/skybin/extracted_secrets/`
- Patterns researched via secrets-patterns-db (1600+ patterns), TruffleHog, Gitleaks

---

## [2.0.0] - 2025-12-05

### Major
- **Rust Telegram Scraper Rewrite** - complete rewrite from Python to Rust for performance
- Unified codebase - all scrapers now in Rust
- Improved error handling and resilience

---

## [1.6.4] - 2025-12-05

### Changed
- Telegram scraper extracts ONLY password files from ALL archives
- Better archive handling for stealer logs

---

## [1.6.3] - 2025-12-05

### Fixed
- Admin panel JavaScript issues
- BruteLogs `.boxed.pw` detection

---

## [1.6.2] - 2025-12-04

### Added
- Admin bulk delete controls in `/x` panel
- Mass source purge functionality

---

## [1.6.1] - 2025-12-04

### Changed
- Removed paste size limit - now accepts up to 100MB
- Better handling of large stealer log dumps

---

## [1.6.0] - 2025-12-04

### Added
- Security hardening: CSP headers, X-Frame-Options, rate limiting
- 24-hour admin session expiration
- SECURITY.md documentation

---

## [1.5.0] - 2025-12-04

### Added
- **Credential summary extraction** - auto-generates titles like "2x API Key, 3x Email:Pass"
- Improved auto-titling for streaming services
- No emojis in generated titles

---

## [1.4.0] - 2025-12-04

### Added
- **40+ Telegram channels** for stealer log monitoring (Daisy Cloud, Bugatti Cloud, Cuckoo Cloud, etc.)
- **Expanded credential patterns** with platform-specific tokens (OpenAI, Stripe, AWS, Firebase, JWT)
- **Keyword-based detection** - 50+ leak keywords for better content identification
- **Lowered filter thresholds** - now accepts single credentials instead of requiring bulk

### Changed
- Minimum email:pass combos: 5 → 1
- Minimum ULP patterns: 3 → 1  
- Minimum content length: 100 → 50 chars
- Keyword threshold: 5 → 3 matches triggers acceptance

### Telegram Channels Added
- Core high-volume: Daisy_Cloud (34M+), bugatti_cloud (16M+), cuckoo_cloud (14M+), StarLinkCloud (2.9M+)
- Additional: LOG_SYNC, HUBHEAD_LOGS, Zeuscloudfree, Wooden_Cloud, MariaLogs, MOONLOGSFREE, EnotLogs, PremCloud, bender_cloud, HelloKittyCloud, brutelogs, bradmax_cloud, sigmcloud, smokercloud, and 20+ more

### Sources Researched
- Paste sites: pastesio.com (API), dpaste.org, rentry.co, bpa.st, ix.io
- Intelligence: SOCRadar, KELA, Group-IB, 8BitSecurity, Webz.io research

---

## [1.3.3] - 2025-12-03

### Added
- Credential validation module (`src/validator.rs`)
- Tor-based paste monitoring (`src/scrapers/tor_pastes.rs`)
- Admin analytics dashboard at `/x`
- Database schema v003 with `scraper_stats` and `activity_logs` tables

### Fixed
- GitHub Gists content fetching
- Ideone scraper pagination

---

## [1.3.0] - 2025-12-01

### Added
- 24 paste sources enabled
- Professional `/status` page with real-time monitoring
- Clickable sources widget with live status
- Admin bulk delete functionality
- Anonymous comments on pastes
- Export to JSON/CSV
- Combo list validator
- Entropy scoring
- Language detection (17+ languages)
- Proxy rotation support

### Detection Patterns
- 41 patterns for credentials, tokens, API keys
- Smart auto-titling for streaming services
- Platform-specific credential detection

---

## [1.2.0] - 2025-11-28

### Added
- Full-text search with SQLite FTS5
- REST API endpoints
- Rate limiting with governor

---

## [1.1.0] - 2025-11-25

### Added
- Multi-source scraping (Pastebin, GitHub Gists, psbdmp)
- Pattern detection for credentials
- Admin authentication

---

## [1.0.0] - 2025-11-20

### Initial Release
- Basic paste aggregation
- SQLite storage
- Web UI with Askama templates
