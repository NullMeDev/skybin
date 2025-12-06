# SkyBin v2.1.0

Paste aggregator that monitors public paste sites and Telegram stealer log channels for leaked credentials, API keys, and sensitive data.

**Live:** https://skybin.lol | https://bin.nullme.lol

## What's New in v2.1.0

- **Secret Extraction Pipeline** - 35+ secret categories with 80+ regex patterns
- **Per-secret deduplication** - SHA256(type+value) prevents duplicate processing
- **Categorized output files** - AWS_Keys.txt, Discord_Tokens.txt, Email_Pass_Combos.txt, etc.
- **Server secret exclusion** - `.excluded_secrets` file prevents broadcasting your own secrets
- **Rust `secret_extractor` module** - unified extraction across all scrapers

### Previous Highlights (v2.0.0)
- **Rust Telegram Scraper** - complete rewrite from Python for performance
- **40+ Telegram channels** - stealer log clouds (Daisy, Bugatti, Cuckoo, StarLink, etc.)
- **Security hardening** - CSP headers, rate limiting, 24hr session expiry

## Features

- **Secret extraction pipeline** with 35+ categories and deduplication
- **Telegram scraper** with 40+ stealer log channels
- Scrapes 24+ paste sites every 30 seconds
- 80+ detection patterns for credentials, tokens, API keys
- Smart auto-titling (identifies Disney+, Netflix, Spotify logins, etc.)
- **Anonymous comments** on pastes - no login required
- **Export to JSON/CSV** for offline analysis
- **Combo list validator** - validates email:password format
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
