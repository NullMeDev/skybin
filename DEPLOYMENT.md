# Deployment Guide

This guide covers deploying SkyBin in various environments.

## Prerequisites

- Rust 1.70+ or precompiled binary
- SQLite 3.x
- 512MB+ RAM
- 1GB+ disk space (depends on retention configuration)

## Quick Start

### 1. Download and Extract

```bash
# Download latest release from GitHub
wget https://github.com/NullMeDev/skybin/releases/download/v0.1.0/paste-vault

# Make executable
chmod +x paste-vault
```

Or build from source:

```bash
git clone git@github.com:NullMeDev/skybin.git
cd skybin
cargo build --release
./target/release/paste-vault
```

### 2. Configure

```bash
# Edit configuration
nano config.toml

# Key settings:
# - server.host: 0.0.0.0 (accessible externally) or 127.0.0.1 (local only)
# - server.port: 8080 (change to desired port)
# - storage.retention_days: 7 (how long to keep pastes)
# - storage.max_pastes: 10000 (maximum pastes in database)
```

### 3. Run

```bash
# Development
cargo run

# Release binary
./target/release/paste-vault

# With debug logging
RUST_LOG=debug ./target/release/paste-vault

# With trace logging for specific module
RUST_LOG=paste_vault=trace ./target/release/paste-vault
```

## Docker Deployment

### Build Docker Image

```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y sqlite3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/paste-vault /usr/local/bin/
COPY --from=builder /app/config.toml /app/
WORKDIR /app
EXPOSE 8080
CMD ["paste-vault"]
```

Build and run:

```bash
docker build -t skybin:latest .
docker run -d \
  -p 8080:8080 \
  -v $(pwd)/data:/app/data \
  -e RUST_LOG=info \
  skybin:latest
```

## Systemd Service

### Create Service File

```bash
sudo tee /etc/systemd/system/skybin.service > /dev/null << EOF
[Unit]
Description=SkyBin - Paste Vault Aggregator
After=network.target

[Service]
Type=simple
User=skybin
WorkingDirectory=/opt/skybin
ExecStart=/opt/skybin/paste-vault
Restart=on-failure
RestartSec=10
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
EOF
```

### Enable and Start

```bash
# Create user
sudo useradd -m -s /bin/false skybin

# Copy binary and config
sudo mkdir -p /opt/skybin
sudo cp target/release/paste-vault /opt/skybin/
sudo cp config.toml /opt/skybin/

# Set permissions
sudo chown -R skybin:skybin /opt/skybin

# Enable service
sudo systemctl enable skybin
sudo systemctl start skybin

# Check status
sudo systemctl status skybin
```

## Environment Variables

Instead of modifying config.toml, use environment variables:

```bash
# Server
export SERVER_HOST="0.0.0.0"
export SERVER_PORT="8080"

# Database
export STORAGE_DB_PATH="/var/lib/skybin/pastevault.db"
export STORAGE_RETENTION_DAYS="7"

# Scraping
export SCRAPING_INTERVAL_SECONDS="300"
export SCRAPING_CONCURRENT_SCRAPERS="5"

# APIs (if using)
export APIS_PASTEBIN_API_KEY="your_key"
export APIS_GITHUB_TOKEN="your_token"
```

Note: Environment variable support requires code changes; see config.rs for implementation.

## Nginx Reverse Proxy

### Configuration

```nginx
upstream skybin {
    server 127.0.0.1:8080;
}

server {
    listen 80;
    server_name paste.example.com;

    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name paste.example.com;

    ssl_certificate /etc/letsencrypt/live/paste.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/paste.example.com/privkey.pem;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Compression
    gzip on;
    gzip_types text/plain text/css application/json application/javascript;

    location / {
        proxy_pass http://skybin;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    location /api {
        proxy_pass http://skybin;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        
        # API-specific timeout
        proxy_read_timeout 30s;
    }
}
```

## SSL/TLS with Let's Encrypt

```bash
# Install certbot
sudo apt-get install certbot python3-certbot-nginx

# Generate certificate
sudo certbot certonly --standalone -d paste.example.com

# Auto-renewal
sudo systemctl enable certbot.timer
sudo systemctl start certbot.timer
```

## Database Maintenance

### Backup

```bash
# Manual backup
cp pastevault.db pastevault.db.backup

# Automated daily backup with cron
0 2 * * * cp /opt/skybin/pastevault.db /backups/pastevault.db.$(date +\%Y\%m\%d)
```

### Cleanup

```bash
# Reset database (WARNING: deletes all data)
rm pastevault.db pastevault.db-wal pastevault.db-shm
# Database auto-initializes on next run
```

## Monitoring

### Health Check

```bash
curl http://localhost:8080/api/health

# Expected response:
{"status":"ok","version":"0.1.0"}
```

### Logging

```bash
# View logs
sudo journalctl -u skybin -f

# Filter for errors
sudo journalctl -u skybin | grep ERROR
```

### Performance

```bash
# Monitor database size
du -h pastevault.db

# Check paste count
sqlite3 pastevault.db "SELECT COUNT(*) FROM pastes;"

# View recent pastes
sqlite3 pastevault.db "SELECT id, source, created_at FROM pastes ORDER BY created_at DESC LIMIT 10;"
```

## Troubleshooting

### Port Already in Use

```bash
# Find process using port 8080
lsof -i :8080

# Kill process
kill -9 <PID>

# Or change port in config.toml
```

### Database Locked

```bash
# This indicates concurrent write attempts
# Solutions:
# 1. Reduce concurrent_scrapers in config
# 2. Upgrade to PostgreSQL (future)
# 3. Increase write queue buffer

# Check for WAL files
ls -la pastevault.db*

# Remove if stuck
rm pastevault.db-wal pastevault.db-shm
# Restart service
```

### High Memory Usage

```bash
# Check memory
free -h

# Reduce max_paste_size in config.toml
# Or reduce concurrent_scrapers

# Monitor with
watch -n 1 'ps aux | grep paste-vault'
```

## Scaling

### Single Server

- Suitable for: Testing, low traffic (<100 requests/min)
- CPU: 1-2 cores minimum
- RAM: 512MB - 2GB
- Storage: 10GB+ (depends on retention)

### Multi-Instance (Load Balanced)

For higher traffic, deploy multiple instances:

```bash
# Instance 1
SERVER_PORT=8081 ./paste-vault

# Instance 2
SERVER_PORT=8082 ./paste-vault

# Instance 3
SERVER_PORT=8083 ./paste-vault
```

Then use load balancer (Nginx):

```nginx
upstream skybin_cluster {
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;
    server 127.0.0.1:8083;
}

server {
    listen 80;
    location / {
        proxy_pass http://skybin_cluster;
    }
}
```

**Note**: All instances need independent databases, or migrate to PostgreSQL.

### Database Upgrade Path

Current SQLite limitation: single writer. For scaling:

1. Current (0.1.0): SQLite with single instance
2. Future (0.2.0): PostgreSQL support for multi-instance
3. Future (0.3.0): Distributed scraping architecture

## Security Hardening

### 1. Firewall

```bash
# UFW example
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp  # SSH
sudo ufw allow 80/tcp  # HTTP
sudo ufw allow 443/tcp # HTTPS
sudo ufw enable
```

### 2. AppArmor/SELinux

Configure AppArmor profile for restricted access:

```bash
sudo aa-enforce /etc/apparmor.d/usr.local.bin.paste-vault
```

### 3. File Permissions

```bash
# Secure database
chmod 600 pastevault.db
chown skybin:skybin pastevault.db

# Secure config
chmod 600 config.toml
chown skybin:skybin config.toml
```

## Performance Tuning

### SQLite

```toml
[storage]
# Increase for better write performance
# (but higher memory usage)
db_cache_pages = 10000
```

### Network

```toml
[scraping]
# Adjust based on system capacity
concurrent_scrapers = 5
jitter_min_ms = 500
jitter_max_ms = 5000
```

## Uninstall

```bash
# Stop service
sudo systemctl stop skybin
sudo systemctl disable skybin

# Remove service
sudo rm /etc/systemd/system/skybin.service

# Remove user
sudo userdel -r skybin

# Remove application
sudo rm -rf /opt/skybin
```

---

For questions, see CONTRIBUTING.md or open an issue on GitHub.
