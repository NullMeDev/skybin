# 🧪 Live Testing Guide - PasteVault v0.1.0

**Status**: ✅ All commits pushed to GitHub
**Binary**: Ready at `target/release/paste-vault` (7.9MB)
**Tests**: 71/71 passing
**Tag**: v0.1.0 pushed to GitHub

---

## Quick Start for Live Testing

### 1. Verify Binary

```bash
# Binary location
ls -lh /home/null/Desktop/paste-vault/target/release/paste-vault
# Expected: 7.9M paste-vault

# Test binary runs
/home/null/Desktop/paste-vault/target/release/paste-vault --version
# Or just try running it
```

### 2. Run Application

```bash
# Make sure config.toml exists
cd /home/null/Desktop/paste-vault
cat config.toml  # Verify configuration

# Run the application
./target/release/paste-vault

# Expected output:
# ✓ Configuration loaded
# ✓ Database initialized at pastevault.db
# ✓ Rate limiter configured
# ✓ Pattern detector initialized with X patterns
# ✓ Scraper task spawned with 300 second interval
# ✓ Web server state created
# ✓ Router configured
# 
# ✅ PasteVault v0.1.0 initialized successfully!
#    🌐 Server listening on http://0.0.0.0:8080
#    📊 Data retention: 7 days
#    ⏱️  Scrape interval: 300 seconds
#
#    Press Ctrl+C to stop the server
```

### 3. Test Web Interface

Open in browser: `http://localhost:8080`

Expected pages:
- ✅ `/` - Dashboard with feed
- ✅ `/search` - Search page
- ✅ `/upload` - Upload page
- ✅ `/api/health` - Health check

### 4. Test API Endpoints

```bash
# Health check
curl http://localhost:8080/api/health
# Expected: {"status":"ok","version":"0.1.0"}

# Get recent pastes
curl http://localhost:8080/api/pastes?limit=5
# Expected: {"success":true,"data":[...],"error":null}

# Get statistics
curl http://localhost:8080/api/stats
# Expected: {"success":true,"data":{...},"error":null}

# Test upload
curl -X POST http://localhost:8080/api/upload \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Test Paste",
    "content": "This is a test paste for live testing",
    "syntax": "plaintext"
  }'
# Expected: {"success":true,"data":"<paste-id>","error":null}

# Retrieve uploaded paste
curl http://localhost:8080/api/paste/<paste-id>
# Expected: {"success":true,"data":{...},"error":null}
```

### 5. Monitor Scraper Activity

Check logs while running:

```bash
# In another terminal
tail -f /home/null/Desktop/paste-vault/pastevault.db* 2>/dev/null || echo "Waiting for database..."

# Look for "Fetched X pastes" messages in console
# These should appear every 5 minutes (300 seconds)
```

### 6. Verify Database

```bash
# Check database was created
ls -lh /home/null/Desktop/paste-vault/pastevault.db*

# Query pastes
sqlite3 /home/null/Desktop/paste-vault/pastevault.db "SELECT COUNT(*) as total, COUNT(CASE WHEN is_sensitive=1 THEN 1 END) as sensitive FROM pastes;"

# Check sources
sqlite3 /home/null/Desktop/paste-vault/pastevault.db "SELECT source, COUNT(*) as count FROM pastes GROUP BY source;"
```

---

## Testing Checklist

### Functionality Tests

- [ ] **Web Interface**
  - [ ] Dashboard loads at `/`
  - [ ] Search page accessible at `/search`
  - [ ] Upload page accessible at `/upload`
  - [ ] Can see recent pastes on dashboard

- [ ] **Upload Functionality**
  - [ ] Can submit paste via web form
  - [ ] Can submit via API (`POST /api/upload`)
  - [ ] Title anonymization works (verify PII removed)
  - [ ] Content hash deduplication works (upload same content twice)
  - [ ] 7-day TTL applied to uploads

- [ ] **Scrapers**
  - [ ] Pastebin scraper fetching data (check logs)
  - [ ] Pastes appearing in database
  - [ ] Data persisting across restarts
  - [ ] Rate limiting preventing abuse

- [ ] **Search & Filtering**
  - [ ] Full-text search working (`/api/search?query=...`)
  - [ ] Source filtering working
  - [ ] Sensitive content filtering working

- [ ] **Pattern Detection**
  - [ ] Pastes with API keys marked as sensitive
  - [ ] Credit cards detected
  - [ ] Emails detected
  - [ ] Private keys detected

### Anonymization Tests

- [ ] **Author Field**
  - [ ] Uploaded pastes have no author
  - [ ] Scraped pastes have no author
  - [ ] API never returns author field

- [ ] **URL Stripping**
  - [ ] Upload URLs removed from URL field
  - [ ] Scraper URLs removed
  - [ ] API returns empty/null for URL

- [ ] **Title Sanitization**
  - [ ] Emails removed from titles
  - [ ] URLs removed from titles
  - [ ] @mentions removed from titles
  - [ ] Domains removed

### Performance Tests

- [ ] **Response Times**
  - [ ] Dashboard loads < 1 second
  - [ ] API endpoints respond < 200ms
  - [ ] Search queries complete < 500ms

- [ ] **Memory Usage**
  - [ ] Check `ps aux | grep paste-vault`
  - [ ] Verify < 256MB RAM usage

- [ ] **Database Performance**
  - [ ] Database size reasonable (< 100MB)
  - [ ] No lock warnings
  - [ ] Queries fast

### Security Tests

- [ ] **Privacy Verification**
  - [ ] Database query shows no author values
  - [ ] No IP addresses logged anywhere
  - [ ] No user tracking cookies
  - [ ] Privacy policy accurate

- [ ] **Input Validation**
  - [ ] Can't upload empty paste
  - [ ] Can't upload > 1MB (default)
  - [ ] Invalid content types handled

- [ ] **Rate Limiting**
  - [ ] Multiple uploads from same IP slow down
  - [ ] Scraper doesn't hammer APIs

---

## Expected Test Results

### Initial State (Fresh Start)
- Database: Empty (0 pastes)
- Scrapers: Starting to fetch in background
- Web server: Listening on 8080

### After 5 Minutes
- Database: ~10-50 pastes from scrapers
- Scrapers: Completed first cycle
- Sensitive: Some pastes marked as sensitive

### After 1 Hour
- Database: 100+ pastes
- Scrapers: Multiple cycles completed
- Search: Working on accumulated data

### After 24 Hours
- Database: 1000+ pastes (if running continuously)
- Rate limits: Preventing API bans
- Auto-purge: Not needed yet (7-day retention)

---

## Troubleshooting Live Testing

### Application Won't Start

```bash
# Check config exists
[ -f config.toml ] && echo "Config OK" || echo "Missing config.toml"

# Try running directly
./target/release/paste-vault

# Check for port conflicts
lsof -i :8080

# Check permissions
ls -l target/release/paste-vault
chmod +x target/release/paste-vault
```

### Scrapers Not Running

```bash
# Check logs for scraper task
# Look for "Fetched X pastes" messages
# Should appear every 300 seconds (5 minutes)

# Check Pastebin API is accessible
curl "https://scrape.pastebin.com/api_scraping.php?limit=1"

# Check GitHub API is accessible
curl "https://api.github.com/gists/public?per_page=1"
```

### Database Issues

```bash
# Check database file
ls -lh pastevault.db*

# Check for locks
rm pastevault.db-wal pastevault.db-shm 2>/dev/null || true

# Restart application
```

### Upload Not Working

```bash
# Check web server is running
curl http://localhost:8080/api/health

# Check upload endpoint
curl -v -X POST http://localhost:8080/api/upload \
  -H "Content-Type: application/json" \
  -d '{"title":"test","content":"test","syntax":"plaintext"}'

# Check error response
```

### High Memory Usage

```bash
# Check current usage
ps aux | grep paste-vault

# Reduce concurrent scrapers in config.toml
[scraping]
concurrent_scrapers = 2  # Reduce from 5

# Restart application
```

---

## Live Testing Stages

### Stage 1: Basic Functionality (30 minutes)
1. Start application
2. Access web interface
3. Test health check
4. Submit one test paste
5. Verify in database

### Stage 2: Scraper Verification (10 minutes)
1. Wait 5 minutes for scraper cycle
2. Check logs for "Fetched" message
3. Verify pastes in database
4. Check pattern detection

### Stage 3: Extended Testing (1+ hours)
1. Monitor for errors in logs
2. Test search functionality
3. Upload multiple pastes
4. Monitor memory/performance
5. Check database growth

### Stage 4: Security Verification (30 minutes)
1. Query database for PII
2. Verify no author values
3. Verify URLs stripped
4. Check title sanitization
5. Verify anonymization working

---

## Performance Monitoring

### Real-Time Monitoring

```bash
# In separate terminals:

# Terminal 1: Run application
./target/release/paste-vault

# Terminal 2: Monitor memory
watch -n 1 'ps aux | grep paste-vault | grep -v grep'

# Terminal 3: Monitor database growth
watch -n 10 'ls -lh pastevault.db*'

# Terminal 4: Monitor logs
tail -f /dev/null  # Or use systemd if available
```

### Database Statistics

```bash
# Total pastes
sqlite3 pastevault.db "SELECT COUNT(*) FROM pastes;"

# Sensitive pastes
sqlite3 pastevault.db "SELECT COUNT(*) FROM pastes WHERE is_sensitive=1;"

# By source
sqlite3 pastevault.db "SELECT source, COUNT(*) FROM pastes GROUP BY source ORDER BY COUNT(*) DESC;"

# Database size
du -h pastevault.db*
```

---

## Success Criteria

✅ **Must Pass**
- [ ] Application starts without errors
- [ ] Web interface accessible
- [ ] Health check endpoint returns 200
- [ ] API upload working
- [ ] Database storing pastes
- [ ] All 71 tests still passing

⚠️ **Should Pass**
- [ ] Scrapers fetching data
- [ ] Pattern detection working
- [ ] Search functionality working
- [ ] Anonymization verified
- [ ] Memory usage < 256MB
- [ ] Response times < 200ms

📊 **Optional Monitoring**
- [ ] Monitor error logs
- [ ] Track database growth
- [ ] Verify rate limiting
- [ ] Check scraper cycles

---

## GitHub Actions (When Enabled)

Once repository is live on GitHub, CI/CD will:
1. Run all tests on push
2. Build release binary
3. Verify code quality
4. Check security

For now, testing is manual.

---

## Next Steps After Testing

1. ✅ Fix any issues found
2. ✅ Update documentation based on findings
3. ✅ Commit fixes to GitHub
4. ✅ Deploy to production server
5. ✅ Enable HTTPS/SSL
6. ✅ Set up monitoring
7. ✅ Announce public availability

---

**Ready to test? Start the application and access http://localhost:8080**

For issues, check logs and refer to DEPLOYMENT.md and CODE_REVIEW.md.
