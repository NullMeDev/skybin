# PasteVault

A high-performance, memory-safe paste aggregator and anonymous pastebin built in Rust. PasteVault continuously monitors multiple public paste sites for sensitive data leaks while providing a secure platform for anonymous paste sharing.

## Features

### 🔍 Multi-Source Scraping
- **Concurrent scraping** of 10+ paste sites (Pastebin, GitHub Gists, Paste.ee, Slexy, Dpaste, and more)
- **Smart rate limiting** with jitter and exponential backoff
- **Automatic deduplication** using SHA256 content hashing
- **Configurable scrape intervals** (default: 5 minutes)

### 🎯 Pattern Detection
- **API Keys**: AWS, GitHub, Stripe, and generic API key patterns
- **Credentials**: Database connection strings, email:password combos
- **Private Keys**: SSH keys, PGP keys, X.509 certificates
- **Financial Data**: Credit card numbers with Luhn validation
- **Network Data**: IP addresses and CIDR ranges
- **Custom Patterns**: User-defined regex patterns with severity levels

### 💾 Smart Storage
- **SQLite with FTS5** for full-text search
- **Auto-purge**: 7-day retention (configurable)
- **FIFO enforcement**: 10,000 paste limit
- **Indexed queries** for fast retrieval

### 🌐 Web Interface
- **Real-time feed** with HTMX auto-refresh
- **Anonymous paste submission**
- **Syntax highlighting** for code
- **Full-text search** with filters
- **Dark theme** (mobile-responsive)
- **Raw text view** for easy copying

## Architecture

```
┌─────────────────────────────────────────┐
│        Multiple Paste Sources           │
│  (Pastebin, Gists, Paste.ee, etc.)     │
└───────────────┬─────────────────────────┘
                │
                ↓
┌───────────────────────────────────────────┐
│      Concurrent Async Scrapers            │
│  • Rate limiting • Deduplication          │
│  • Jitter • Proxy support                 │
└───────────────┬───────────────────────────┘
                │
                ↓
┌───────────────────────────────────────────┐
│        Pattern Detection Engine           │
│  • Regex-based • Luhn validation          │
│  • CIDR matching • Custom rules           │
└───────────────┬───────────────────────────┘
                │
                ↓
┌───────────────────────────────────────────┐
│      SQLite Database (FTS5)               │
│  • Auto-purge • FIFO cap • Indexes        │
└───────────────┬───────────────────────────┘
                │
                ↓
┌───────────────────────────────────────────┐
│        Axum Web Server                    │
│  • Feed • Search • Upload • Raw view      │
└───────────────────────────────────────────┘
```

## Quick Start

### Prerequisites
- Rust 1.70+ (we use 1.84.1)
- SQLite 3.35+ (bundled with rusqlite)

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/paste-vault
cd paste-vault

# Build release binary
cargo build --release

# Copy config
cp config.toml.example config.toml

# Edit configuration (optional)
nano config.toml

# Run
./target/release/paste-vault
```

### Configuration

Edit `config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080

[storage]
retention_days = 7
max_pastes = 10000

[scraping]
interval_seconds = 300  # 5 minutes

[sources]
pastebin = true
gists = true
paste_ee = true

[patterns]
aws_keys = true
credit_cards = true
emails = true
```

### First Run

```bash
# The database will be automatically initialized
./target/release/paste-vault

# Access the web interface
# Open http://localhost:8080 in your browser
```

## Usage

### Web Interface

- **`/`** - Real-time feed of discovered pastes
- **`/paste/{id}`** - View individual paste with highlighting
- **`/raw/{id}`** - Raw text view
- **`/search`** - Full-text search with filters
- **`/upload`** - Submit anonymous paste

### API Endpoints

```bash
# Get recent pastes (JSON)
curl http://localhost:8080/api/pastes?limit=10

# Create new paste
curl -X POST http://localhost:8080/api/paste \
  -H "Content-Type: application/json" \
  -d '{"title":"Test","content":"Hello world","syntax":"plaintext"}'
```

## Deployment

### Systemd Service

```bash
# Create service file
sudo nano /etc/systemd/system/pastevault.service
```

```ini
[Unit]
Description=PasteVault - Paste Aggregator and Anonymous Pastebin
After=network.target

[Service]
Type=simple
User=pastevault
WorkingDirectory=/opt/pastevault
ExecStart=/opt/pastevault/paste-vault
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start
sudo systemctl enable pastevault
sudo systemctl start pastevault
sudo systemctl status pastevault

# View logs
sudo journalctl -u pastevault -f
```

## Development

### Project Structure

```
paste-vault/
├── src/
│   ├── main.rs           # Entry point
│   ├── config.rs         # Configuration parser
│   ├── models.rs         # Data structures
│   ├── db.rs             # Database layer
│   ├── hash.rs           # Content hashing
│   ├── patterns/         # Pattern detection
│   │   ├── mod.rs
│   │   ├── detector.rs
│   │   └── rules.rs
│   ├── scrapers/         # Source scrapers
│   │   ├── mod.rs
│   │   ├── traits.rs
│   │   ├── pastebin.rs
│   │   ├── gists.rs
│   │   └── ...
│   ├── scheduler.rs      # Scraping orchestrator
│   ├── rate_limiter.rs   # Rate limiting
│   └── web/              # Web interface
│       ├── mod.rs
│       ├── routes.rs
│       ├── handlers.rs
│       └── templates/
├── migrations/           # SQL migrations
├── static/               # CSS, JS, assets
├── config.toml           # Configuration
└── Cargo.toml
```

### Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Adding New Paste Sources

1. Implement the `Scraper` trait in `src/scrapers/`
2. Add source configuration to `config.toml`
3. Register scraper in `src/scrapers/mod.rs`

Example:

```rust
pub struct MySiteScraper;

#[async_trait]
impl Scraper for MySiteScraper {
    fn name(&self) -> &str {
        "mysite"
    }

    async fn fetch_recent(&self, client: &Client) -> Result<Vec<DiscoveredPaste>> {
        // Implementation
    }
}
```

## Security Considerations

- **No authentication required** - This is intentional for anonymous usage
- **Rate limiting** - Per-IP limits on paste submission
- **Content size limits** - Configurable max paste size
- **HTML escaping** - All user input is sanitized
- **No script execution** - Static templates only
- **Auto-purge** - 7-day retention minimizes data exposure

## Performance

- **Memory-safe**: Rust's ownership system prevents memory leaks
- **Concurrent**: Tokio async runtime for efficient I/O
- **Fast queries**: SQLite FTS5 with proper indexing
- **Single binary**: ~10MB compiled size
- **Low resource usage**: ~50MB RAM typical

## Troubleshooting

### Scrapers failing

```bash
# Check logs
RUST_LOG=debug ./paste-vault

# Test individual source
# (Implementation detail - add debug endpoints)
```

### Database locked

```bash
# SQLite is single-writer - ensure only one instance runs
pkill paste-vault
rm pastevault.db-wal pastevault.db-shm
```

### High memory usage

```bash
# Reduce concurrent scrapers in config.toml
[scraping]
concurrent_scrapers = 3

# Reduce max pastes
[storage]
max_pastes = 5000
```

## Roadmap

- [ ] **v0.1**: Core functionality (M0-M2 complete)
- [ ] **v0.2**: Web UI and search (M3-M4)
- [ ] **v0.3**: Production hardening (M5)
- [ ] **v1.0**: Stable release
- [ ] **v1.1**: Additional sources (Rentry, Hastebin, etc.)
- [ ] **v1.2**: Advanced pattern detection (ML-based)
- [ ] **v2.0**: Distributed scraping, PostgreSQL support

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request

## License

MIT License - see LICENSE file

## Credits

Inspired by:
- [PasteHunter](https://github.com/kevthehermit/PasteHunter) by kevthehermit (Python implementation)
- [pastebin_scraper](https://github.com/FireFart/pastebin_scraper) by FireFart (Go implementation)

Built with:
- [Rust](https://www.rust-lang.org/) - Memory-safe systems programming
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SQLite](https://www.sqlite.org/) - Embedded database
- [HTMX](https://htmx.org/) - Dynamic UI without JavaScript complexity
- [Tailwind CSS](https://tailwindcss.com/) - Utility-first CSS

## Support

- **Issues**: https://github.com/yourusername/paste-vault/issues
- **Discussions**: https://github.com/yourusername/paste-vault/discussions

---

**⚠️ Legal Notice**: This tool is intended for security research and monitoring your own infrastructure for data leaks. Always respect websites' Terms of Service and robots.txt. The authors are not responsible for misuse of this software.
