# SkyBin Security Audit & Implementation Roadmap

## ✅ COMPLETED: Critical Security Fixes (Dec 6, 2025)

### 1. Path Traversal Vulnerabilities - **FIXED**
- **src/config.rs:260** - Added canonical path validation + `.toml` extension check
- **src/proxy.rs:105** - Added canonical path validation + `.txt` extension check  
- **telegram-scraper-rs/src/config.rs:63** - Added canonical path + extension validation
- **telegram-scraper-rs/src/telegram.rs:38** - Added session file path validation (must be in current_dir)

**Commit:** 3705fe7 - "SECURITY: Fix path traversal vulnerabilities"

---

## ⚠️ REMAINING CRITICAL ISSUES

### 2. File Inclusion Attack in telegram-scraper/scraper.py
**Location:** `scraper.py` lines with `open()` calls (706, 714, 895, 908, 918)

**Issue:** Potential arbitrary file read when processing archive contents

**Fix Required:**
```python
# Before (VULNERABLE):
with open(tmp_path, 'wb') as f:
    f.write(data)

# After (SECURE):
import os
# Validate tmp_path is within TEMP_DIR
canonical_tmp = os.path.realpath(tmp_path)
canonical_temp_dir = os.path.realpath(TEMP_DIR)
if not canonical_tmp.startswith(canonical_temp_dir):
    raise ValueError(f"Security: Temp file outside allowed directory")
    
with open(canonical_tmp, 'wb') as f:
    f.write(data)
```

### 3. aiohttp Vulnerability Audit
**Location:** telegram-scraper/scraper.py (lines 23, 706, 714, 1098, 1156, 1190, 1215, etc.)

**Action Required:**
1. Check aiohttp version: `pip show aiohttp`
2. Update to latest (4.x): `pip install --upgrade aiohttp>=4.0.0`
3. Review all `ClientSession()` usages for:
   - Timeout settings (prevent DOS)
   - SSL verification enabled
   - No user-controlled URLs without validation

### 4. GitHub Actions Security
**Location:** `.github/workflows/*.yml`

**Issue:** 3rd party actions should be pinned to commit SHAs

**Fix:** Pin all actions to specific commits:
```yaml
# Before:
- uses: actions/checkout@v4

# After:
- uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11  # v4.1.1
```

---

## 📋 FEATURE IMPLEMENTATION ROADMAP

Due to the massive scope (20+ features), implementation is broken into phases:

### Phase 1: Core Infrastructure (Est: 4-6 hours)
- [ ] 3-Tier Deduplication System
  - Fuzzy hash matching (ssdeep library)
  - Credential fingerprinting
  - Cross-source duplicate groups
- [ ] Database schema updates for deduplication
- [ ] Admin panel deduplication metrics

### Phase 2: New Paste Site Scrapers (Est: 3-4 hours)
- [ ] Paste.ee (pastee.dev fallback domain)
- [ ] Hastebin instances (multiple domains)
- [ ] Rentry.co (RSS feed parser)
- [ ] Snippet.host (HTML scraper)
- [ ] 0bin instances (JSON API)
- [ ] Live source status tracking (health checks)
- [ ] Sources dropdown with green/gray dots

### Phase 3: Telegram Enhancements (Est: 3-4 hours)
- [ ] Extract all joined channels from session
- [ ] 2-tier priority system (15s / 60s intervals)
- [ ] Auto-discovery module (scan descriptions, pinned messages)
- [ ] Channel health monitoring (admin panel only)
- [ ] Posts/hour tracking
- [ ] Credential hit rate metrics
- [ ] Auto-disable dead channels

### Phase 4: Search & Filtering (Est: 2-3 hours)
- [ ] Integrate pattern filters into search
- [ ] Syntax: `type:api-key`, `type:password`, `type:token`
- [ ] No separate UI clutter
- [ ] Trending patterns widget (compact)
- [ ] Export CSV/JSON (clean button)

### Phase 5: Real-time Features (Est: 2-3 hours)
- [ ] WebSocket implementation
- [ ] Live paste feed updates
- [ ] Minimal UI impact
- [ ] Telegram webhook to group ID 5000998639

### Phase 6: Title Improvements (Est: 1-2 hours)
- [ ] More detailed credential summaries
- [ ] Keep current format, enhance specificity
- [ ] Example: "5x Gmail (weak passwords), 3x GitHub PAT, 12x Netflix (premium accounts)"

### Phase 7: Poster Self-Delete (Est: 1-2 hours)
- [ ] Track poster by session/IP hash
- [ ] Allow delete own pastes only
- [ ] Add "Delete" button on paste detail (if poster matches)

### Phase 8: GraphQL API (Decision Pending)
**Explanation:**
GraphQL provides more flexible queries than REST. Instead of:
- `GET /api/pastes?source=telegram&limit=50`
- `GET /api/paste/123`
- `GET /api/search?q=netflix`

You'd have ONE endpoint (`POST /graphql`) where clients query exactly what they need:
```graphql
query {
  pastes(source: "telegram", limit: 50) {
    id
    title
    matchedPatterns { type severity }
  }
}
```

**Pros:**
- Clients fetch only needed fields (reduces bandwidth)
- Single endpoint (simpler routing)
- Type-safe queries
- Better for mobile apps

**Cons:**
- More complex backend (need GraphQL resolver library)
- Caching is harder
- Slightly higher server overhead

**Recommendation:** **Skip for now**. Current REST API is sufficient unless you're building a mobile app or complex frontend.

---

## 🎨 UI IMPROVEMENTS (Approved)

### Implemented:
- ✅ Removed Search/Upload tabs from navigation
- ✅ Added favicon (blue gradient with "S")
- ✅ Disclaimer page updated

### Remaining:
- [ ] Real-time feed (WebSocket)
- [ ] Trending patterns widget (compact, non-intrusive)
- [ ] Export button (CSV/JSON, minimal UI)
- [ ] Sources dropdown with live status indicators
- [ ] Admin panel: API rate limits, channel health, source status

### Rejected (as requested):
- ❌ Dark/Light theme toggle
- ❌ Syntax highlighting themes
- ❌ Compact/Comfortable view toggle
- ❌ Customizable feed columns
- ❌ Infinite scroll
- ❌ Skeleton loading
- ❌ Animated counters
- ❌ Toast notifications
- ❌ Keyboard shortcuts
- ❌ Paste preview on hover
- ❌ Date range picker
- ❌ Multiple source selection UI
- ❌ Severity checkboxes
- ❌ File size range filter
- ❌ Saved searches
- ❌ Smart search suggestions
- ❌ Related pastes
- ❌ Breach correlation (HIBP)
- ❌ Domain grouping
- ❌ Threat intelligence tags
- ❌ Auto-redaction (explicitly forbidden)
- ❌ Progressive Web App
- ❌ Lazy load images
- ❌ Bulk operations
- ❌ Paste versioning
- ❌ RSS/Atom feeds

---

## ⏱️ ESTIMATED TOTAL IMPLEMENTATION TIME

**Total:** 16-24 hours of focused development work

**Priority Order:**
1. **Security fixes** (remaining): 1-2 hours ✅ CRITICAL
2. **Deduplication system**: 4-6 hours
3. **New paste scrapers**: 3-4 hours
4. **Telegram enhancements**: 3-4 hours
5. **Search/UI features**: 2-3 hours
6. **Real-time WebSocket**: 2-3 hours
7. **Misc (title improvements, self-delete)**: 2-3 hours

---

## 🔧 MCP SERVERS USED

Based on the requirements, the following MCP servers would be useful:

1. **firecrawl_search** - For discovering new paste sites and leak sources
2. **brave_web_search** - For researching paste site APIs and documentation
3. **github (MCP)** - For managing PRs and issues during development
4. **perplexity_research** - For deep research on paste site APIs and Telegram best practices

---

## 📝 NEXT STEPS

1. **Immediate:** Fix remaining security issues (#2, #3, #4 above)
2. **Phase 1:** Implement deduplication system (most requested)
3. **Phase 2:** Add new paste scrapers (high value)
4. **Phase 3:** Telegram enhancements
5. **Phase 4-7:** UI and feature improvements

**Recommendation:** Work through phases linearly, committing after each phase for safety.
