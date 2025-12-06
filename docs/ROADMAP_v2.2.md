# SkyBin v2.2.0 Feature Roadmap

Generated: December 6, 2025

## Overview

This roadmap outlines 27+ new features and scraping sources beyond the existing Telegram integration, prioritized by implementation effort and security research value.

---

## New Paste/Code Sources (12)

### 1. PrivateBin Instances
**Effort:** Medium | **Value:** High
- Scrape public PrivateBin instances (self-hosted encrypted pastes)
- Target: known public instances + configurable custom URLs
- Challenge: Some use client-side encryption (need shared key)

### 2. Pastery.net
**Effort:** Low | **Value:** Medium
- Simple paste site with API
- No authentication required for public pastes

### 3. Nekobin.com
**Effort:** Low | **Value:** Medium
- Code-focused paste site popular in Asian developer communities
- JSON API available

### 4. CodeShare.io
**Effort:** Medium | **Value:** Medium
- Real-time code collaboration platform
- Public rooms may contain leaked code

### 5. TextBin.net
**Effort:** Low | **Value:** Low
- Simple text paste service
- No API, HTML scraping required

### 6. Paste.Mozilla.org
**Effort:** Medium | **Value:** High
- Mozilla's official paste service
- May contain internal dev notes, configs

### 7. Katb.in / Katbin.com
**Effort:** Low | **Value:** Medium
- Minimalist paste service
- No API, HTML scraping

### 8. Pastes.io (Enhanced)
**Effort:** Low | **Value:** Medium
- Already exists but can add user profile scraping
- Trending pastes feed

### 9. Toffeeshare
**Effort:** Medium | **Value:** Medium
- P2P file sharing with paste functionality
- Requires browser automation

### 10. ZeroBin Instances
**Effort:** High | **Value:** High
- Multiple public ZeroBin deployments
- Client-side encrypted (need link with key)

### 11. Hastebin Alternatives Network
**Effort:** Low | **Value:** Medium
- toptal.com/developers/hastebin
- hastebin.skyra.pw
- paste.gg
- bin.disroot.org

### 12. CryptoPaste Services
**Effort:** Medium | **Value:** High
- Various crypto-focused paste sites
- Often used for wallet dumps

---

## Threat Intel Sources (8)

### 13. BreachForums RSS/API
**Effort:** High | **Value:** Critical
- Monitor new posts via RSS or scraping
- Requires Tor/proxy rotation
- Challenge: Frequent domain changes

### 14. Exposed Databases RSS
**Effort:** Medium | **Value:** High
- Monitor exposed.lol, databases.today
- Aggregate leaked database announcements

### 15. Doxbin Monitor
**Effort:** High | **Value:** Medium
- Track new dox posts
- Tor access required

### 16. Discord Webhook Leaks
**Effort:** Medium | **Value:** High
- Scan for exposed Discord webhook URLs
- Test validity and categorize by server type

### 17. XSS.is / HackForums RSS
**Effort:** High | **Value:** High
- Underground forum monitoring
- Requires registration/Tor

### 18. LeakBase/Snusbase Monitors
**Effort:** Medium | **Value:** High
- Track new breach announcements
- API integration where available

### 19. Telegram Channel Aggregator (Enhanced)
**Effort:** Medium | **Value:** Critical
- Extend existing Telegram scraper
- Auto-discover new leak channels
- Cross-channel deduplication

### 20. IRC/Matrix Channel Monitor
**Effort:** High | **Value:** Medium
- Monitor public channels on Libera.Chat, Matrix servers
- Bot-based monitoring

---

## Cloud/Infrastructure Scanners (5)

### 21. S3 Bucket Scanner
**Effort:** High | **Value:** Critical
- Detect misconfigured public S3 buckets
- Parse discovered bucket contents
- Integration with existing credential detection

### 22. GCS Bucket Scanner
**Effort:** High | **Value:** High
- Google Cloud Storage public bucket detection
- Similar to S3 scanner

### 23. Azure Blob Scanner
**Effort:** High | **Value:** High
- Azure storage container enumeration
- Check for public access

### 24. Elasticsearch Exposed Instance Scanner
**Effort:** Medium | **Value:** Critical
- Detect publicly accessible ES clusters
- Common source of massive data leaks

### 25. Jenkins/CI Open Instance Scanner
**Effort:** Medium | **Value:** High
- Find exposed Jenkins instances
- Often contain secrets in build configs

---

## Platform Features (7)

### 26. Real-Time Alerts (WebSocket/SSE)
**Effort:** Medium | **Value:** High
```rust
// Add to web/mod.rs
async fn sse_events(State(db): State<Database>) -> Sse<impl Stream<Item = Event>> {
    // Stream new paste notifications
}
```
- Live notifications for new credentials matching patterns
- Filter by severity, type, keyword

### 27. Webhook Notifications
**Effort:** Low | **Value:** High
- Discord webhook integration
- Slack incoming webhooks
- Custom HTTP POST endpoints
- Configurable triggers (severity, pattern match)

### 28. Bulk Export API
**Effort:** Low | **Value:** Medium
```
GET /api/export?format=json&from=DATE&to=DATE
GET /api/export?format=csv&pattern=aws_keys
```
- JSON/CSV/STIX export formats
- Date range filtering
- Pattern-based filtering

### 29. Duplicate Clustering
**Effort:** High | **Value:** High
- Group similar leaks (same breach, different sources)
- MinHash/LSH similarity detection
- UI grouping view

### 30. Credential Validation Service (Opt-in)
**Effort:** High | **Value:** Critical
- Check if credentials are still active
- Email deliverability check
- API key validity test (rate-limited)
- **Ethics:** Only for authorized security research

### 31. STIX/TAXII Threat Feed
**Effort:** Medium | **Value:** High
- Export credentials in STIX 2.1 format
- TAXII server for automated consumption
- Integration with threat intel platforms

### 32. Pattern Scoring ML Model
**Effort:** High | **Value:** Medium
- Train model on credential quality
- Score by: freshness, uniqueness, target value
- Prioritize high-value findings

---

## UI Improvements

### Current: Static HTML served by Axum
### Recommended: HTMX + Alpine.js upgrade

**Benefits:**
- Progressive enhancement
- No build step required
- Server-side rendering maintained
- Real-time updates without full page reload

**Implementation:**
1. Add HTMX CDN to base.html
2. Replace fetch() calls with hx-* attributes
3. Add Alpine.js for client-side state
4. Implement SSE endpoint for live updates

**Example:**
```html
<!-- Current -->
<div id="pastes"></div>
<script>fetch('/api/pastes').then(...)</script>

<!-- With HTMX -->
<div hx-get="/api/pastes" hx-trigger="load, every 30s" hx-swap="innerHTML">
  Loading...
</div>
```

---

## Implementation Priority

### Phase 1 (v2.2.0) - Week 1
1. ✅ Simplified titles (implemented)
2. ✅ URL classifier (implemented)
3. Webhook notifications
4. Bulk export API

### Phase 2 (v2.3.0) - Week 2-3
5. Real-time alerts (SSE)
6. HTMX UI upgrade
7. Pastery.net scraper
8. Nekobin scraper

### Phase 3 (v2.4.0) - Week 4+
9. BreachForums monitor
10. S3 bucket scanner
11. Elasticsearch scanner
12. STIX/TAXII export

### Phase 4 (v3.0.0) - Future
13. Duplicate clustering
14. ML pattern scoring
15. Credential validation

---

## MCP Servers Used

This roadmap was generated with assistance from:
- `firecrawl_search` - Research on paste sites and security tools
- `brave_web_search` - Alternative search for threat intel sources
- `context7` - Documentation lookup for Rust web frameworks
- `perplexity_ask` - Comet browser recommendations integration

---

## Security Considerations

1. **Rate Limiting:** All scanners must respect target rate limits
2. **Tor/Proxy:** Dark web sources require Tor integration
3. **Legal:** Bucket scanners should only check for public access, not attempt unauthorized access
4. **Ethics:** Credential validation must be opt-in and used responsibly
5. **Storage:** Consider data retention policies for sensitive findings
