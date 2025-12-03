# SkyBin

Paste aggregator that monitors public paste sites for leaked credentials, API keys, and sensitive data.

**Live:** https://bin.nullme.lol

## Features

- Scrapes multiple paste sites every 60 seconds
- Pattern detection for credentials, API keys, tokens
- Full-text search with SQLite FTS5
- Auto-generated titles based on content analysis
- Anonymous paste submission
- REST API

## Detected Patterns

- AWS access keys and secrets
- GitHub/GitLab tokens
- Slack/Discord tokens
- Database connection strings
- Private keys (RSA, DSA, EC)
- Email:password combos
- Credit card numbers
- Bearer tokens
- Streaming service credentials

## Sources

- Pastebin
- GitHub Gists
- Slexy
- ControlC
- Pastecode
- Dpaste.org
- Hastebin
- Defuse
- Codepad
- External URLs (manual submission)

## API

```
GET  /api/pastes          - list recent pastes
GET  /api/paste/:id       - get paste details
GET  /api/search?q=       - full-text search
GET  /api/stats           - statistics
GET  /api/health          - health check
POST /api/paste           - create paste
POST /api/submit-url      - submit URL to scrape
```

## Config

Edit `config.toml`:

```toml
[scraping]
interval_seconds = 60
concurrent_scrapers = 8

[sources]
pastebin = true
gists = true
slexy = true
controlc = true
```

## Build

```bash
cargo build --release
./target/release/paste-vault
```

## License

MIT
