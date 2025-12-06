#!/bin/bash
set -e

# SkyBin v2.5.0 Deployment Script
# Usage: ./deploy.sh [server-user@server-ip]

SERVER=${1:-"root@your-server-ip"}
REMOTE_DIR="/opt/skybin"
SERVICE_NAME="skybin"

echo "🚀 SkyBin v2.5.0 Deployment"
echo "=========================="
echo "Target: $SERVER"
echo "Remote directory: $REMOTE_DIR"
echo ""

# Build release binary
echo "📦 Building release binary..."
cargo build --release

# Check if binary exists
if [ ! -f "target/release/skybin" ]; then
    echo "❌ Error: Binary not found at target/release/skybin"
    exit 1
fi

echo "✓ Binary built successfully ($(ls -lh target/release/skybin | awk '{print $5}'))"

# Create deployment package
echo "📋 Creating deployment package..."
TEMP_DIR=$(mktemp -d)
mkdir -p "$TEMP_DIR/skybin"

# Copy binary
cp target/release/skybin "$TEMP_DIR/skybin/"

# Copy static files
cp -r static "$TEMP_DIR/skybin/"
cp -r templates "$TEMP_DIR/skybin/" 2>/dev/null || echo "Note: No templates directory"

# Copy config template
cp config.toml "$TEMP_DIR/skybin/config.toml.example"

# Copy telegram scraper (Python)
cp -r telegram-scraper "$TEMP_DIR/skybin/"

# Create systemd service file
cat > "$TEMP_DIR/skybin.service" <<'EOF'
[Unit]
Description=SkyBin Paste Aggregator v2.5.0
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/skybin
ExecStart=/opt/skybin/skybin
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Environment
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
EOF

# Create telegram scraper service file
cat > "$TEMP_DIR/skybin-telegram.service" <<'EOF'
[Unit]
Description=SkyBin Telegram Scraper
After=network.target skybin.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/skybin/telegram-scraper
ExecStart=/usr/bin/python3 scraper.py
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

echo "✓ Deployment package created"

# Create deployment tarball
echo "📦 Creating tarball..."
cd "$TEMP_DIR"
tar -czf skybin-v2.5.0.tar.gz skybin/ skybin.service skybin-telegram.service
cd - > /dev/null

echo "✓ Tarball created: $(ls -lh $TEMP_DIR/skybin-v2.5.0.tar.gz | awk '{print $5}')"

# Upload to server
echo "📤 Uploading to server..."
scp "$TEMP_DIR/skybin-v2.5.0.tar.gz" "$SERVER:/tmp/"

# Deploy on server
echo "🔧 Deploying on server..."
ssh "$SERVER" bash <<'ENDSSH'
set -e

echo "Stopping services..."
systemctl stop skybin 2>/dev/null || true
systemctl stop skybin-telegram 2>/dev/null || true

echo "Extracting package..."
cd /tmp
tar -xzf skybin-v2.5.0.tar.gz

echo "Backing up old installation..."
if [ -d "/opt/skybin" ]; then
    mv /opt/skybin "/opt/skybin.backup.$(date +%Y%m%d-%H%M%S)"
fi

echo "Installing new version..."
mv skybin /opt/
chmod +x /opt/skybin/skybin

echo "Preserving config if exists..."
if [ -f "/opt/skybin.backup.*/config.toml" ]; then
    cp /opt/skybin.backup.*/config.toml /opt/skybin/ 2>/dev/null || true
fi

echo "Preserving database if exists..."
if [ -f "/opt/skybin.backup.*/skybin.db" ]; then
    cp /opt/skybin.backup.*/skybin.db /opt/skybin/ 2>/dev/null || true
fi

echo "Preserving .env if exists..."
if [ -f "/opt/skybin.backup.*/telegram-scraper/.env" ]; then
    cp /opt/skybin.backup.*/telegram-scraper/.env /opt/skybin/telegram-scraper/ 2>/dev/null || true
fi

echo "Installing systemd services..."
mv skybin.service /etc/systemd/system/
mv skybin-telegram.service /etc/systemd/system/
systemctl daemon-reload

echo "Starting services..."
systemctl enable skybin
systemctl enable skybin-telegram
systemctl start skybin
sleep 2
systemctl start skybin-telegram

echo "Checking status..."
systemctl status skybin --no-pager -l
systemctl status skybin-telegram --no-pager -l

echo "Cleaning up..."
rm /tmp/skybin-v2.5.0.tar.gz

echo "✓ Deployment complete!"
echo ""
echo "Service status:"
systemctl is-active skybin && echo "  ✓ skybin: running" || echo "  ✗ skybin: stopped"
systemctl is-active skybin-telegram && echo "  ✓ telegram: running" || echo "  ✗ telegram: stopped"
ENDSSH

# Cleanup
rm -rf "$TEMP_DIR"

echo ""
echo "✅ Deployment successful!"
echo ""
echo "Next steps:"
echo "  1. Verify config.toml on server: ssh $SERVER 'cat /opt/skybin/config.toml'"
echo "  2. Check logs: ssh $SERVER 'journalctl -u skybin -f'"
echo "  3. Test endpoint: curl http://your-server/api/health"
echo ""
