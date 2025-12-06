# Changelog

All notable changes to SkyBin will be documented in this file.

## [2.5.0] - 2025-12-06

### Added
- **Phase 7: Poster Self-Delete** - Short-lived paste URLs with deletion tokens
  - **Deletion Tokens**: UUID v4 tokens generated for all user uploads
  - **Secure Delete Endpoint**: `DELETE /api/delete/:token` with token validation
  - **deletion_tokens Table**: Store paste_id to token mapping with CASCADE delete
  - **Response Fields**: deletion_token and deletion_url returned on paste creation
  - **No Tokens for**: Scraped pastes or staff posts (user uploads only)
- **Phase 6: Export Features** - Bulk JSON/CSV export for search results
  - **Bulk JSON Export**: `/api/export/bulk/json?q=...` with search filters
  - **Bulk CSV Export**: `/api/export/bulk/csv?q=...` with metadata
  - **Export Limit**: 1000 pastes max per export (configurable)
  - **CSV Format**: id, title, source, syntax, is_sensitive, high_value, created_at, content_preview
  - **Search Integration**: Export buttons in advanced search UI (search_v2.html)
  - **Timestamped Filenames**: skybin-export-YYYYMMDD-HHMMSS.{json|csv}
- **Phase 5: Real-time WebSocket Feed** - Live paste streaming with filters and notifications
  - **WebSocket Endpoint**: `/api/ws` with query param filters (sensitive_only, high_value_only, source)
  - **RealtimeBroadcast System**: Tokio broadcast channels with 1000-message buffer
  - **Event Types**: PasteAdded, PasteViewed, StatsUpdate, Ping (heartbeat)
  - **Connection Tracking**: Active WebSocket client count monitoring
  - **Live Feed UI**: Animated real-time paste stream at `/live`
  - **Client-side Filters**: All/Sensitive/Critical with pause/resume controls
  - **Audio Notifications**: Optional beep alerts for critical pastes
  - **Auto-reconnect**: 3-second reconnect on disconnect
  - **Performance**: Max 100 items displayed, slideIn animations
- **Phase 4: Advanced Search Improvements** - Enhanced search with modern UX features
  - **Autocomplete Search Suggestions**: Real-time query suggestions from patterns, sources, and search history
  - **Saved Searches**: Client-side localStorage persistence for frequently used searches with custom labels
  - **Advanced Filters Panel**: Severity, date range, pattern type, source, and sensitive-only filters
  - **Search History Tracking**: Backend support for search analytics and popular queries
  - **Enhanced SearchFilters**: Added severity, created_after/before, pattern fields for precise filtering
  - **Search Suggestions API**: `/api/search/suggestions?q=` endpoint with categorized results
  - **Advanced Search UI**: Standalone search_v2.html with toolbar, filter chips, and keyboard navigation
- **3-Tier Deduplication System** - Advanced multi-level duplicate detection
  - **Tier 1**: Exact content hash matching (normalized SHA256) - unchanged
  - **Tier 2**: Near-duplicate detection via SimHash with Hamming distance threshold
    - Sliding window of 500 recent pastes for comparison
    - Configurable Hamming distance threshold (default: ≤6 bits)
    - Catches minor edits, whitespace changes, and slight variations
  - **Tier 3**: Per-secret gating for near-duplicates
    - Extracts and deduplicates individual secrets within similar content
    - Only stores pastes with new, unseen secrets
    - Writes new secrets to categorized files even if paste is dropped
- **5 New Paste Site Scrapers** - Expanded coverage to 35+ sources
  - **PasteFS** (pastefs.com) - Simple paste site with recent feed API
  - **Kbinbin** (kbinbin.com) - Paste aggregator with HTML parsing
  - **Snippet** (snippet.host) - Minimalist paste service
  - **PrivateBin** (privatebin.net) - Encrypted pastebin with directory endpoint
  - **ZeroBin** (0bin.net/zerobin.net) - Multi-instance encrypted paste discovery
- **Telegram Scraper Enhancements** - Channel prioritization and auto-discovery
  - **Channel Prioritization**: Dynamic scoring based on credential yield (40%), success rate (30%), member count (15%), recency (15%)
  - **Auto-Discovery**: Keyword-based search to find new leak channels automatically
  - **Health Tracking**: Per-channel metrics with success rate and activity monitoring
  - **Priority Scoring**: 0-100 scale with weighted factors for optimal channel selection

### Changed
- Scheduler now preserves raw data (no auto-redaction by default)
- Near-duplicate detection triggers per-secret extraction pipeline
- Dedup metrics available for future admin dashboard integration

### Technical
- **Phase 7 Changes**:
  - Database schema v006 with deletion_tokens table
  - New DB methods: store_deletion_token(), delete_paste_by_token()
  - CreatePasteResponse extended with Optional deletion_token and deletion_url
  - UUID v4 token generation on user upload (not for scraped/staff pastes)
  - Cascade delete ensures tokens removed when paste deleted
- **Phase 6 Features**:
  - Two new bulk export endpoints: export_bulk_json(), export_bulk_csv()
  - Reuses SearchFilters for flexible export criteria
  - Export buttons added to search_v2.html toolbar
  - Timestamped filenames with chrono formatting
- **Phase 5 Modules**:
  - New `src/realtime.rs` module with WebSocket broadcast system (254 lines)
  - Axum WebSocket support with `futures` for stream handling
  - RealtimeBroadcast with Arc<RwLock> connection tracking
  - Event filtering at WebSocket layer (sensitive_only, high_value_only, source)
  - Broadcast on paste upload in handlers.rs
  - Tests: test_broadcast_basic, test_connection_tracking, test_filter_sensitive
- **Phase 4 Modules**:
  - New `src/search_history.rs` module with SQLite-backed search tracking
  - SearchFilters now Serializable for saved searches
  - Dynamic SQL query builder with optional filters
  - Autocomplete suggestion engine with pattern/source/recent categorization
- New `src/dedup.rs` module with fast 64-bit SimHash implementation
- Added `fxhash` dependency for non-cryptographic hashing
- Added `base64` dependency for PrivateBin decoding
- Config fields added: `pastefs`, `kbinbin`, `snippet`, `privatebin`, `zerobin`
- All tests passing including new simhash and search_history unit tests
- Telegram: `channel_manager.py` for prioritization and auto-discovery

---

## [2.4.1] - 2025-12-06

### Security
- **CRITICAL: Fixed path traversal vulnerabilities** in telegram scraper
  - Added path canonicalization for all file operations in `scraper.py`
  - Validated temp file paths are within allowed directories
  - Protected against malicious archive contents escaping temp directories
  - Fixed file inclusion attack vectors in `credential_extractor.py`
- **Updated aiohttp dependency** from 3.9.0 to 4.0.0+ for latest security patches
- **Pinned GitHub Actions to commit SHAs** to prevent supply chain attacks
  - `actions/checkout@v4` → `@11bd7190` (v4.2.2)
  - `actions/upload-artifact@v4` → `@6f51ac03` (v4.4.3)
  - `actions/upload-pages-artifact@v3` → `@56afc609` (v3.0.1)
  - `actions/deploy-pages@v4` → `@d6db9016` (v4.0.5)
  - `Swatinem/rust-cache@v2` → `@e207df5d` (v2.7.5)
  - Exception: `dtolnay/rust-toolchain@stable` intentionally not pinned (trusted maintainer)

### Documentation
- Added `.github/ACTION_PINS.md` with action pinning reference and update schedule
- Added `SECURITY_AUDIT.md` documenting all security fixes and remaining work

### Technical
- All file reads now use `os.path.realpath()` with directory prefix validation
- Canonical path checks prevent symlink traversals and path escapes
- Archive extraction validates all file paths before writing

---

## [2.4.0] - 2025-12-06

### Added
- **Intelligent Credential Classification** - Automatic service detection and smart titling
  - Classifies credentials by email domain (Gmail, Outlook, Yahoo, etc.)
  - Identifies URL-based credentials (Netflix, Spotify, PayPal, Steam, etc.)
  - Generates titles like "5x Gmail Logins" or "10x Netflix Accounts"
  - Prepends credential summary to paste content for quick overview
- **File Upload with VirusTotal Scanning** - Secure file upload system
  - Support for files up to 400MB (configurable)
  - Optional VirusTotal API integration for malware scanning
  - Scans files up to 650MB using VT's large file upload endpoint
  - Automatic rejection of malicious content
  - Async scanning before database operations (no blocking)
- **Staff Badge System** - Infrastructure for verified staff posts
  - Database schema with staff_badge column (TEXT)
  - Optional staff badge display (e.g., "SkyBin Owner", "Developer")
  - Database migration script (004_add_staff_badge.sql)
  - All existing pastes default to NULL (regular posts)
- **Legal Disclaimer Page** - Professional /disclaimer page added
  - Clear OSINT/educational use policy
  - Liability disclaimers and content warnings
  - Accessible from navigation bar

### Changed
- **UI Cleanup** - Removed redundant filter tabs
  - Removed: Alerts, Interesting, Sensitive, Pastebin, Github, Ideone, Telegram tabs
  - Replaced with clean search interface
  - Classification now visible in paste titles
- **Telegram Scraper Posting Threshold** - Lower barrier to entry
  - Reduced from 5 credentials to 1 credential minimum
  - Better capture of single high-value leaks
- **Version Bump** - Updated to 2.4.0 across all components
  - Cargo.toml manifests
  - Static HTML files
  - Navigation and footer

### Technical
- Added `classifier` module for service-based credential categorization
- Added `virustotal` module with VirusTotalClient for file scanning
- Database schema v004 with high_value and staff_badge columns
- Multipart form support in reqwest dependency
- Config options: max_upload_size, enable_virustotal_scan, virustotal_api_key
- All 136 unit tests passing

---

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
