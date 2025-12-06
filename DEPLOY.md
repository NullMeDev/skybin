# SkyBin Deployment (v2.5.0)

> Note: This file was consolidated. See `DEPLOYMENT.md` for the authoritative, up-to-date guide for v2.5.0.

## Quick Start

✅ All code committed and pushed
✅ Git tag v2.4.0 created
✅ 136 tests passing
✅ Clean build (release mode)
✅ CHANGELOG.md updated
✅ Website metadata updated

## Configuration Updates Required

Add to `/opt/skybin/config.toml`:

```toml
[server]
host = "127.0.0.1"
port = 8082
max_paste_size = 1_000_000
max_upload_size = 419430400  # 400MB (optional, defaults to 400MB)
enable_virustotal_scan = false  # Set to true if you have VT API key

[apis]
# ... existing keys ...
virustotal_api_key = ""  # Optional: Add your VirusTotal API key
```

## Database Migration

Run migration to add staff_badge column:

```bash
ssh vps
cd /opt/skybin
sqlite3 skybin.db < source/migrations/004_add_staff_badge.sql
```

Or manually:
```sql
ALTER TABLE pastes ADD COLUMN staff_badge TEXT DEFAULT NULL;
ALTER TABLE pastes ADD COLUMN high_value INTEGER DEFAULT 0;
UPDATE metadata SET value = '004' WHERE key = 'schema_version';
```

## Legacy steps (v2.4.x)

These historical steps have been superseded by `DEPLOYMENT.md`. If you are deploying v2.5.0 or later, use that guide. Leaving legacy notes below for reference.

1. **Build release binary locally:**
```bash
cd /home/null/Desktop/skybin
cargo build --release
strip target/release/skybin  # Optional: reduce binary size
```

2. **Backup current deployment:**
```bash
ssh vps "cd /opt/skybin && cp skybin skybin.backup-$(date +%Y%m%d-%H%M) && cp skybin.db skybin.db.backup"
```

3. **Stop service:**
```bash
ssh vps "sudo systemctl stop skybin"
```

4. **Upload new binary:**
```bash
rsync -avz --progress target/release/skybin vps:/opt/skybin/skybin
```

5. **Upload static files:**
```bash
rsync -avz --progress --delete static/ vps:/opt/skybin/static/
```

6. **Upload migration script:**
```bash
rsync -avz --progress migrations/ vps:/opt/skybin/source/migrations/
```

7. **Run database migration:**
```bash
ssh vps "cd /opt/skybin && sqlite3 skybin.db 'ALTER TABLE pastes ADD COLUMN IF NOT EXISTS staff_badge TEXT DEFAULT NULL;'"
ssh vps "cd /opt/skybin && sqlite3 skybin.db 'ALTER TABLE pastes ADD COLUMN IF NOT EXISTS high_value INTEGER DEFAULT 0;'"
ssh vps "cd /opt/skybin && sqlite3 skybin.db \"UPDATE metadata SET value = '004' WHERE key = 'schema_version';\""
```

8. **Start service:**
```bash
ssh vps "sudo systemctl start skybin"
```

9. **Verify deployment:**
```bash
ssh vps "systemctl status skybin"
ssh vps "curl -s http://127.0.0.1:8082/api/health | jq"
```

10. **Check logs:**
```bash
ssh vps "sudo journalctl -u skybin -n 50 --no-pager"
```

## Quick Deploy Script

```bash
#!/bin/bash
set -e

echo "Building v2.4.0..."
cargo build --release

echo "Stopping service..."
ssh vps "sudo systemctl stop skybin"

echo "Backing up..."
ssh vps "cd /opt/skybin && cp skybin skybin.old && cp skybin.db skybin.db.backup"

echo "Uploading binary..."
rsync -avz target/release/skybin vps:/opt/skybin/

echo "Uploading static files..."
rsync -avz --delete static/ vps:/opt/skybin/static/

echo "Running migration..."
ssh vps "cd /opt/skybin && sqlite3 skybin.db 'ALTER TABLE pastes ADD COLUMN IF NOT EXISTS staff_badge TEXT DEFAULT NULL; ALTER TABLE pastes ADD COLUMN IF NOT EXISTS high_value INTEGER DEFAULT 0;'"

echo "Starting service..."
ssh vps "sudo systemctl start skybin"

echo "Checking status..."
sleep 2
ssh vps "systemctl status skybin --no-pager"

echo "✅ Deployment complete!"
```

## Rollback Plan

If issues occur:

```bash
ssh vps "sudo systemctl stop skybin"
ssh vps "cd /opt/skybin && cp skybin.old skybin"
ssh vps "sudo systemctl start skybin"
```

## New Features to Test

1. **Staff Badge System** - Access `/x` admin panel, go to "Staff Post" tab
2. **File Upload** - Test uploading files up to 400MB
3. **Smart Titles** - Verify credential posts have proper titles like "5x Gmail Logins"
4. **Legal Disclaimer** - Check `/disclaimer` page exists

## Configuration Reference

### New Config Options

- `server.max_upload_size` - Max file upload size (bytes), default 400MB
- `server.enable_virustotal_scan` - Enable VT scanning, default false
- `apis.virustotal_api_key` - Your VT API key (optional)

### VirusTotal Setup (Optional)

1. Get free API key from https://www.virustotal.com/gui/my-apikey
2. Add to config.toml:
```toml
[server]
enable_virustotal_scan = true

[apis]
virustotal_api_key = "your_key_here"
```

## Post-Deployment Verification

- [ ] Service running: `systemctl status skybin`
- [ ] Health check: `curl http://127.0.0.1:8082/api/health`
- [ ] Admin panel accessible at `/x`
- [ ] Staff Post tab visible in admin panel
- [ ] Disclaimer page at `/disclaimer`
- [ ] Version shows v2.4.0 in footer/header
- [ ] No errors in logs: `journalctl -u skybin -n 100`
