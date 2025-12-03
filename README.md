# SkyBin

Paste aggregator that monitors public paste sites for leaked credentials, API keys, and sensitive data.

**Live:** https://skybin.lol | https://bin.nullme.lol

## Features

- Scrapes 18 paste sites every 45 seconds
- 50+ detection patterns for credentials, tokens, API keys
- Smart auto-titling (identifies Disney+, Netflix, Spotify logins, etc.)
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

## Sources (18)

Pastebin, GitHub Gists, Ideone, Bpa.st, Pastecode, dpaste.org, Defuse, Codepad, Slexy, ControlC, Hastebin, Ghostbin, Ubuntu Pastebin, ix.io, JustPaste, Rentry, dpaste.com, External URLs

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

## Build

```bash
cargo build --release
./target/release/paste-vault
```

## License

MIT
