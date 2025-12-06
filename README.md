# SkyBin v2.5.0

Paste aggregator that monitors public paste sites and Telegram stealer log channels for leaked credentials, API keys, and sensitive data.

**Live:** https://skybin.lol | https://bin.nullme.lol

## What's New in v2.5.0

### Phase 7: Poster Self-Delete
- **Deletion tokens** - UUID v4 tokens for user-uploaded pastes
- **Self-service deletion** - Delete your paste via unique URL (no account needed)
- **Secure endpoint** - `DELETE /api/delete/:token` with validation

### Phase 6: Bulk Export
- **JSON/CSV export** - Export up to 1000 pastes with search filters
- **Timestamped files** - `skybin-export-YYYYMMDD-HHMMSS.{json|csv}`
- **Search integration** - Export buttons in advanced search UI

### Phase 5: Real-time WebSocket Feed
- **Live paste stream** - Real-time updates at `/live`
- **WebSocket endpoint** - `wss://skybin.lol/api/ws` with filters
- **Audio alerts** - Optional notifications for critical pastes
- **Client filters** - All/Sensitive/Critical with pause/resume

### Phase 4: Advanced Search
- **Autocomplete suggestions** - Real-time query hints from patterns/sources
- **Saved searches** - Persistent favorites with custom labels
- **Advanced filters** - Severity, date range, pattern type, source filters
- **Search history** - Backend analytics for popular queries

### Previous Highlights (v2.4.0)
- **Smart classification** - Auto-detects Gmail, Netflix, Steam credentials
- **File upload** - Support for 400MB files with VirusTotal scanning
- **Staff badges** - Verified staff posts with custom badges
- **3-tier deduplication** - SimHash near-duplicate detection

## Features

- **🔴 Real-time WebSocket feed** with live paste streaming
- **🔍 Advanced search** with autocomplete and saved searches
- **📦 Bulk export** to JSON/CSV (up to 1000 pastes)
- **🗑️ Self-delete tokens** for user uploads
- **Secret extraction pipeline** with 35+ categories and 80+ patterns
- **Telegram scraper** with 40+ stealer log channels
- Scrapes 35+ paste sites every 30 seconds
- Smart auto-titling (identifies Disney+, Netflix, Spotify logins, etc.)
- **Anonymous comments** on pastes - no login required
- **Entropy scoring** - identifies high-entropy secrets
- **Language detection** - identifies 17+ programming languages
- **Proxy rotation** - distribute scraping across proxies
- Full-text search with SQLite FTS5
- Anonymous paste submission
- REST API

## Detected Patterns

- **Streaming Services:** Disney+, Netflix, Hulu, HBO Max, Spotify, Crunchyroll, etc.
- **Gaming:** Steam, Epic Games, PlayStation, Xbox, Minecraft, Fortnite
- **Cloud:** AWS, Azure, GCP, DigitalOcean, Heroku, Cloudflare
- **Social:** Discord, Telegram, Instagram, Facebook, Twitter, TikTok
- **Email:** Gmail, Outlook, Yahoo, ProtonMail
- **Financial:** Credit cards, PayPal, banking credentials
- **Auth:** OAuth tokens, JWT, Bearer tokens, API keys
- **Infrastructure:** SSH keys, database strings, private keys

## Active Sources

| Source | Status | Rate |
|--------|--------|------|
| Pastebin | ✅ Active | 30/cycle |
| GitHub Gists | ✅ Active | 15/cycle |
| Ideone | ✅ Active | 24/cycle |

*Most paste sites don't have public APIs - submit URLs via `/api/submit-url`*

## API

```
GET  /api/pastes                - list recent pastes
GET  /api/paste/:id             - get paste details
GET  /api/paste/:id/comments    - get comments
GET  /api/export/:id/json       - export paste as JSON
GET  /api/export/:id/csv        - export paste as CSV
GET  /api/search?q=             - full-text search
GET  /api/stats                 - statistics
GET  /api/health                - health check
POST /api/paste                 - create paste
POST /api/paste/:id/comments    - add comment
POST /api/submit-url            - submit URL to scrape
```

## Build

```bash
cargo build --release
./target/release/skybin
```

## License

MIT
