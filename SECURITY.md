# SkyBin Security Audit & Hardening

**Audit Date:** December 5, 2025  
**Auditor:** NullMe  
**Version:** 1.6.0 → 1.6.1 (Security Hardened)

## Executive Summary

A comprehensive security audit was performed on the SkyBin codebase, including the Rust backend, web frontend, and Python Telegram scraper. This document outlines the findings, implemented fixes, and remaining recommendations.

---

## ✅ IMPLEMENTED FIXES

### 1. Security Headers (CRITICAL)

**Issue:** No Content-Security-Policy, X-Frame-Options, or other security headers were set.

**Fix:** Added tower-http security header middleware in `src/web/mod.rs`:

```rust
.layer(SetResponseHeaderLayer::overriding(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
.layer(SetResponseHeaderLayer::overriding(header::X_FRAME_OPTIONS, "DENY"))
.layer(SetResponseHeaderLayer::overriding(header::X_XSS_PROTECTION, "1; mode=block"))
.layer(SetResponseHeaderLayer::overriding(header::REFERRER_POLICY, "no-referrer"))
.layer(SetResponseHeaderLayer::overriding(header::CONTENT_SECURITY_POLICY, "default-src 'self'; ..."))
.layer(SetResponseHeaderLayer::overriding("Permissions-Policy", "geolocation=(), microphone=(), camera=(), interest-cohort=()"))
```

**Headers Now Applied:**
- `X-Content-Type-Options: nosniff` - Prevents MIME sniffing
- `X-Frame-Options: DENY` - Prevents clickjacking
- `X-XSS-Protection: 1; mode=block` - XSS filter (legacy browsers)
- `Referrer-Policy: no-referrer` - No referrer sent (anonymity)
- `Content-Security-Policy` - Restricts resource loading
- `Permissions-Policy` - Disables geolocation, microphone, camera, FLoC

---

### 2. API Rate Limiting (CRITICAL)

**Issue:** Public endpoints had no rate limiting, enabling DoS attacks.

**Fix:** Added in-memory sliding window rate limiter in `src/rate_limiter.rs`:

| Endpoint | Rate Limit |
|----------|------------|
| `/api/paste` (upload) | 10/minute |
| `/api/submit-url` | 20/minute |
| `/api/search` | 60/minute |
| `/api/paste/:id/comments` | 10/minute |
| `/api/x/login` (admin) | 5/minute |

**Implementation:**
- `ApiRateLimiter` struct with per-key tracking
- `ApiRateLimiters` pre-configured set for all vulnerable endpoints
- Global key used to preserve user anonymity (no IP tracking)

---

### 3. Admin Session Expiration (HIGH)

**Issue:** Admin tokens never expired - sessions persisted indefinitely.

**Fix:** Modified `src/admin.rs` to add 24-hour TTL:

```rust
const SESSION_TTL: Duration = Duration::from_secs(24 * 60 * 60);

// Sessions stored with creation timestamp
sessions: RwLock<HashMap<String, Instant>>,

// Verification checks TTL
pub fn verify_token(&self, token: &str) -> bool {
    if let Some(created) = sessions.get(token) {
        return created.elapsed() < SESSION_TTL;
    }
    false
}
```

**Additional:**
- Expired sessions automatically cleaned up on new logins
- `cleanup_expired()` method for manual cleanup

---

## ✅ ALREADY SECURE (Confirmed)

### SQL Injection Protection
All database queries use parameterized statements via rusqlite's `params![]` macro. No string interpolation in queries.

### XSS Protection
- JavaScript: `escapeHtml()` function sanitizes user content
- Rust: `html_escape::encode_text()` for comment content
- Templates: Askama auto-escapes by default

### Anonymization
- Authors stripped from all scraped pastes
- URLs anonymized before storage
- Titles sanitized (no emails, URLs)
- Usernames redacted from content
- No IP logging in activity_logs table

### Authentication
- Admin passwords hashed with SHA256
- Session tokens generated with `rand::thread_rng()` (32 bytes)
- Bearer token authentication

### Content Deduplication
- SHA256 content hash with UNIQUE constraint
- Prevents duplicate storage

### Admin Panel
- `<meta name="robots" content="noindex, nofollow">` prevents indexing
- Hidden at `/x` path
- Token-protected API endpoints

---

## ⚠️ RECOMMENDATIONS (Not Implemented)

### 1. Move Secrets to Environment Variables (MEDIUM)
**Current:** `config.toml` contains plaintext:
- Pastebin API key
- GitHub PAT token
- Admin password

**Recommendation:** Use environment variables or a secrets manager:
```toml
[apis]
pastebin_api_key = "${PASTEBIN_API_KEY}"
github_token = "${GITHUB_TOKEN}"

[admin]
password = "${ADMIN_PASSWORD}"
```

### 2. Admin IP Allowlist (LOW)
Consider restricting `/x` panel to specific IPs via nginx:
```nginx
location /x {
    allow 192.168.1.0/24;
    allow 10.0.0.0/8;
    deny all;
    proxy_pass http://127.0.0.1:8082;
}
```

### 3. CSRF Tokens (LOW)
While JSON APIs are less vulnerable to CSRF, consider adding tokens for critical endpoints if browser-based form submissions are ever needed.

### 4. Remove GitHub Attribution for Full Anonymity (INFO)
`static/index.html` links to `github.com/NullMeDev/skybin`. Remove if complete anonymity is desired.

### 5. Version Disclosure (INFO)
Health endpoint returns `CARGO_PKG_VERSION`. Consider removing in production.

---

## Telegram Scraper Security

The Python Telegram scraper (`telegram-scraper/scraper.py`) was also reviewed:

**Secure:**
- Session files stored locally (not committed to git)
- API credentials loaded from `.env` (not in code)
- Content filtered before posting (credential detection)
- Temp files cleaned up after processing

**New Feature (v2.6):**
- Credential summary auto-extraction prepends critical info at top of pastes
- Makes leaked credentials immediately visible without scrolling

---

## Verification

After deployment, verify headers with:
```bash
curl -sI https://skybin.lol/ | grep -iE "x-content-type|x-frame|x-xss|referrer-policy|content-security|permissions"
```

Test rate limiting:
```bash
for i in {1..15}; do curl -s -o /dev/null -w "%{http_code}\n" -X POST https://skybin.lol/api/paste -H "Content-Type: application/json" -d '{"content":"test"}'; done
# Should return 200 for first 10, then 500 (rate limited)
```

---

## Files Modified

| File | Changes |
|------|---------|
| `Cargo.toml` | Added `set-header` feature to tower-http |
| `src/web/mod.rs` | Security headers middleware, rate limiters in AppState |
| `src/web/handlers.rs` | Rate limit checks on vulnerable endpoints |
| `src/admin.rs` | Session TTL (24h), HashMap with timestamps |
| `src/rate_limiter.rs` | `ApiRateLimiter`, `ApiRateLimiters` structs |
| `src/main.rs` | Initialize rate limiters |
| `telegram-scraper/scraper.py` | `extract_credential_summary()` function |

---

## Changelog

### v1.6.1 (2025-12-05)
- Added security headers (CSP, X-Frame-Options, etc.)
- Implemented API rate limiting on all public POST endpoints
- Added 24-hour admin session expiration
- Telegram scraper v2.6: Credential summary prepending
