# SkyBin v2.5.0 Deployment Guide

## Quick Deployment

```bash
# Deploy to your server
./deploy.sh root@your-server-ip

# Or if server is already configured in deploy.sh
./deploy.sh
```

## Manual Deployment Steps

### 1. Build Release Binary

```bash
cargo build --release
```

Binary will be at: `target/release/skybin` (11MB stripped)

### 2. Server Requirements

- **OS**: Ubuntu/Debian Linux
- **Rust**: Not required (binary is statically linked)
- **Python**: 3.8+ (for Telegram scraper)
- **Dependencies**: 
  - `python3-pip` (for Telegram scraper)
  - `systemd` (for service management)

### 3. Directory Structure on Server

```
/opt/skybin/
├── skybin                    # Main Rust binary
├── config.toml               # Configuration file
├── skybin.db                 # SQLite database (auto-created)
├── static/                   # Web UI assets
│   ├── index.html
│   ├── live.html
│   ├── search_v2.html
│   └── ...
├── telegram-scraper/         # Python Telegram scraper
│   ├── scraper.py
│   ├── channel_manager.py
│   ├── .env                  # Telegram API credentials
│   └── requirements.txt
└── extracted_secrets/        # Secret extraction output (auto-created)
```

### 4. Configuration

#### Main Config (`/opt/skybin/config.toml`)

Key settings to configure:

```toml
[server]
host = "0.0.0.0"              # Listen on all interfaces
port = 8080                   # Web server port

[storage]
db_path = "skybin.db"         # SQLite database path
retention_days = 7            # Auto-delete after N days

[scraping]
interval_seconds = 30         # Scrape interval

[admin]
password = "your-secure-password-here"  # Admin panel access

[sources]
# Enable/disable scrapers
pastebin = true
ghostbin = true
telegram = false              # Handled by Python scraper
# ... etc
```

#### Telegram Scraper (`/opt/skybin/telegram-scraper/.env`)

```bash
TELEGRAM_API_ID=your_api_id
TELEGRAM_API_HASH=your_api_hash
TELEGRAM_PHONE=+1234567890
SKYBIN_API_URL=http://127.0.0.1:8080
```

**Note**: You mentioned using the Python scraper for now until you can authenticate with Rust. The Rust binary doesn't include a Telegram scraper - it uses the Python one exclusively.

### 5. Systemd Services

#### SkyBin Main Service

```bash
sudo systemctl start skybin
sudo systemctl enable skybin
sudo systemctl status skybin
```

#### Telegram Scraper Service

```bash
sudo systemctl start skybin-telegram
sudo systemctl enable skybin-telegram
sudo systemctl status skybin-telegram
```

### 6. Logs

```bash
# SkyBin logs
sudo journalctl -u skybin -f

# Telegram scraper logs
sudo journalctl -u skybin-telegram -f

# Both together
sudo journalctl -u skybin -u skybin-telegram -f
```

### 7. Health Check

```bash
curl http://localhost:8080/api/health
```

Expected response:
```json
{
  "status": "ok",
  "version": "2.5.0",
  "database": "connected",
  "url_queue_size": 0,
  "timestamp": 1733491200
}
```

### 8. Firewall Configuration

```bash
# Allow HTTP (if not using reverse proxy)
sudo ufw allow 8080/tcp

# Or if using nginx reverse proxy
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
```

### 9. Nginx Reverse Proxy (Optional)

```nginx
server {
    listen 80;
    server_name skybin.lol;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket support for /api/ws
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### 10. Database Migration

The database schema will auto-upgrade from v005 to v006 on first run. No manual migration needed.

**Schema v006 Changes**:
- Added `deletion_tokens` table for Phase 7 (poster self-delete)

## Features by Phase

### Phase 4: Advanced Search ✅
- **URL**: `/search` (new advanced search UI at `/search_v2.html`)
- **API**: `/api/search/suggestions?q=`
- **Features**: Autocomplete, saved searches, advanced filters

### Phase 5: Real-time Feed ✅
- **URL**: `/live`
- **API**: `/api/ws` (WebSocket endpoint)
- **Features**: Live paste stream, filters, audio alerts

### Phase 6: Export ✅
- **API**: 
  - `/api/export/bulk/json?q=...`
  - `/api/export/bulk/csv?q=...`
- **Features**: Bulk export with search filters (max 1000 pastes)

### Phase 7: Self-Delete ✅
- **API**: `DELETE /api/delete/:token`
- **Features**: User-uploaded pastes get deletion tokens

## Post-Deployment Checklist

- [ ] Binary deployed to `/opt/skybin/skybin`
- [ ] `config.toml` configured with admin password
- [ ] Static files deployed to `/opt/skybin/static/`
- [ ] Telegram scraper `.env` configured (if using)
- [ ] Both systemd services running
- [ ] Health check passes: `curl http://localhost:8080/api/health`
- [ ] Web UI accessible: `http://your-server:8080/`
- [ ] Live feed works: `http://your-server:8080/live`
- [ ] Admin panel accessible: `http://your-server:8080/x`
- [ ] Logs show no errors: `journalctl -u skybin -n 50`

## Troubleshooting

### Service won't start
```bash
# Check logs
sudo journalctl -u skybin -n 100 --no-pager

# Check permissions
ls -la /opt/skybin/skybin
sudo chmod +x /opt/skybin/skybin

# Check config syntax
/opt/skybin/skybin --help  # Shouldn't error on config parse
```

### Database errors
```bash
# Check database file
ls -la /opt/skybin/skybin.db

# If corrupted, backup and delete (will recreate)
sudo mv /opt/skybin/skybin.db /opt/skybin/skybin.db.backup
sudo systemctl restart skybin
```

### Telegram scraper issues
```bash
# Check if Python dependencies installed
cd /opt/skybin/telegram-scraper
pip3 list | grep -E '(telethon|aiohttp)'

# Install if missing
pip3 install -r requirements.txt

# Test scraper manually
python3 scraper.py
```

### WebSocket not connecting
- Check nginx config has WebSocket support (upgrade headers)
- Check firewall allows WebSocket connections
- Check browser console for errors

## Rolling Back

```bash
# Stop services
sudo systemctl stop skybin skybin-telegram

# Restore from backup
sudo mv /opt/skybin.backup.YYYYMMDD-HHMMSS /opt/skybin

# Restart
sudo systemctl start skybin skybin-telegram
```

## Performance Tuning

### Database
- Default max 10,000 pastes (FIFO)
- FTS5 full-text search indexed
- Auto-cleanup of expired pastes

### Rate Limits
- Upload: 10 req/min per IP
- Search: 30 req/min per IP
- Comments: 5 req/min per IP

### Resource Usage
- **Memory**: ~50-100MB typical
- **CPU**: Low (<5% idle, spikes during scrapes)
- **Disk**: Varies by retention (typically 1-5GB)
- **Network**: ~1-10Mbps depending on scraper activity

## Monitoring

```bash
# Service status
systemctl status skybin

# Resource usage
htop -p $(pidof skybin)

# Disk usage
du -sh /opt/skybin/

# Active connections (WebSocket)
ss -tnp | grep skybin

# Database size
ls -lh /opt/skybin/skybin.db
```

## Version Info

- **Version**: 2.5.0
- **Schema**: v006
- **Release Date**: 2025-12-06
- **Rust Version**: 1.80+ (built with)
- **Phases**: 1-7 complete
