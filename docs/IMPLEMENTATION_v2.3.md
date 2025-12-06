# SkyBin v2.3.0 Implementation Guide

**Created:** 2025-12-06  
**Status:** IN PROGRESS  
**Author:** Claude (AI Assistant)

---

## Overview

This document tracks the implementation of SkyBin v2.3.0, which includes:
1. **UI/HTMX Upgrade** - Progressive enhancement with HTMX + Alpine.js
2. **Telegram Scraper Rewrite** - Credential-only posting, proper titles, large file support

---

## Part 1: UI/HTMX Upgrade

### Goals
- Replace vanilla JS fetch() with HTMX for partial page updates
- Add Alpine.js for reactive UI state (modals, filters, toggles)
- Implement infinite scroll instead of pagination buttons
- Add skeleton loaders for better UX
- Add severity-based styling for paste items

### Files to Modify

#### 1. `static/index.html`
**Current state:** Vanilla JS with setInterval auto-refresh, fetch() calls, manual pagination

**Changes needed:**
```html
<!-- Add to <head> -->
<script src="https://unpkg.com/htmx.org@1.9.10"></script>
<script src="https://unpkg.com/alpinejs@3.13.3" defer></script>

<!-- Convert paste container to HTMX -->
<div id="paste-container"
     hx-get="/api/pastes/html"
     hx-trigger="load, every 30s"
     hx-swap="innerHTML">
    <!-- Skeleton loader shown initially -->
</div>

<!-- Convert stats bar to HTMX -->
<div class="stats-bar" 
     hx-get="/api/stats/html"
     hx-trigger="load, every 30s"
     hx-swap="innerHTML">
</div>

<!-- Infinite scroll trigger -->
<div hx-get="/api/pastes/html?page=2"
     hx-trigger="revealed"
     hx-swap="afterend"
     hx-indicator="#loading-indicator">
</div>
```

**Alpine.js for modals:**
```html
<div x-data="{ sourcesOpen: false }">
    <button @click="sourcesOpen = !sourcesOpen">Sources</button>
    <div x-show="sourcesOpen" x-transition @click.outside="sourcesOpen = false">
        <!-- Modal content -->
    </div>
</div>
```

#### 2. `src/web/mod.rs`
**New endpoints needed:**

```rust
// GET /api/pastes/html - Returns HTML fragment of paste list
async fn get_pastes_html(
    State(state): State<AppState>,
    Query(params): Query<PasteListParams>,
    HxRequest(is_htmx): HxRequest,  // from axum-htmx
) -> impl IntoResponse {
    // If HTMX request, return just the paste list HTML
    // If regular request, return full page (for direct navigation)
}

// GET /api/stats/html - Returns HTML fragment of stats bar
async fn get_stats_html(State(state): State<AppState>) -> impl IntoResponse {
    // Return just the stats bar HTML
}

// GET /api/search/html?q= - Returns HTML fragment of search results
async fn get_search_html(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    // Return search results as HTML
}
```

#### 3. `static/styles.css`
**Additions:**

```css
/* Severity-based paste styling */
.paste-item.severity-critical {
    border-left: 4px solid #dc2626;
    background: linear-gradient(90deg, rgba(220, 38, 38, 0.1) 0%, transparent 100%);
}

.paste-item.severity-high {
    border-left: 4px solid #f59e0b;
}

.paste-item.severity-medium {
    border-left: 4px solid #3b82f6;
}

/* Skeleton loaders */
.skeleton {
    background: linear-gradient(90deg, var(--bg-secondary) 25%, var(--bg-tertiary) 50%, var(--bg-secondary) 75%);
    background-size: 200% 100%;
    animation: skeleton-loading 1.5s infinite;
}

@keyframes skeleton-loading {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
}

.skeleton-paste {
    height: 72px;
    border-radius: 8px;
    margin-bottom: 8px;
}

/* HTMX loading indicator */
.htmx-indicator {
    display: none;
}
.htmx-request .htmx-indicator {
    display: block;
}
```

#### 4. `Cargo.toml`
**Add dependency:**
```toml
axum-htmx = "0.6"
```

---

## Part 2: Telegram Scraper Rewrite

### Goals
1. **ONLY post when credentials found** - Skip messages/files with no credentials
2. **Proper titles** - "3x Netflix, 44x Email:Pass" NOT "[TG] channel_name"
3. **Large file support** - Up to 12GB (server has 80GB, 5 concurrent max), streaming downloads
4. **File processing pipeline** - Download → Extract → Categorize → Post → Delete

### Files to Modify

#### 1. `telegram-scraper/scraper.py`

**Configuration changes:**
```python
# OLD
MAX_ARCHIVE_SIZE = 500 * 1024 * 1024  # 500MB
MAX_CONCURRENT_DOWNLOADS = 10

# NEW - 12GB max, 5 concurrent (80GB server)
MAX_ARCHIVE_SIZE = 12 * 1024 * 1024 * 1024  # 12GB
MAX_CONCURRENT_DOWNLOADS = 5  # 5 * 12GB = 60GB max, fits in 80GB server
DOWNLOAD_TIMEOUT = 3600  # 1 hour for large files
DOWNLOAD_DIR = '/tmp/skybin_downloads'  # Temp directory for large files
CHUNK_SIZE = 10 * 1024 * 1024  # 10MB chunks for streaming
```

**Remove `generate_auto_title()` usage:**
```python
# OLD (in post_to_skybin and other places)
if summary_title:
    final_title = summary_title
else:
    final_title = base_title if base_title else "Telegram Leak"

# NEW - Only use credential summary, skip if none
summary_title, summary_header = extract_credential_summary(content)
if not summary_title:
    logger.info("No credentials found, skipping post")
    return False  # Don't post if no credentials
final_title = summary_title
```

**New streaming download function:**
```python
async def stream_download_file(self, message, output_path: str) -> bool:
    """
    Stream download a file to disk with progress logging.
    For files > 100MB, uses chunked download to avoid memory issues.
    """
    try:
        file_size = message.document.size
        downloaded = 0
        last_log = 0
        
        async with aiofiles.open(output_path, 'wb') as f:
            async for chunk in self.client.iter_download(message.document):
                await f.write(chunk)
                downloaded += len(chunk)
                
                # Log progress every 100MB
                if downloaded - last_log > 100 * 1024 * 1024:
                    pct = (downloaded / file_size) * 100
                    logger.info(f"  Download progress: {pct:.1f}% ({downloaded / 1024 / 1024:.1f}MB)")
                    last_log = downloaded
        
        return True
    except Exception as e:
        logger.error(f"Stream download failed: {e}")
        return False
```

**New file processing pipeline:**
```python
async def process_downloaded_file(self, filepath: str, channel_name: str) -> bool:
    """
    Process a downloaded file:
    1. Read content (extract if archive)
    2. Run credential extraction
    3. If credentials found, post to SkyBin
    4. Delete temp file
    """
    try:
        filename = os.path.basename(filepath)
        lower = filename.lower()
        
        # Extract content based on file type
        if any(lower.endswith(ext) for ext in ARCHIVE_EXTENSIONS):
            content = await self.extract_archive_content(filepath)
        else:
            with open(filepath, 'r', errors='ignore') as f:
                content = f.read()
        
        if not content or len(content) < 50:
            logger.info(f"  Empty or too small: {filename}")
            return False
        
        # Extract credentials using the extractor
        from credential_extractor import extract_and_save, get_extractor
        extractor = get_extractor()
        result = extractor.extract(content, source="telegram")
        
        # ONLY post if credentials were found
        if not result.secrets:
            logger.info(f"  No credentials found in: {filename}")
            return False
        
        # Save new secrets to category files
        if result.new_secrets:
            extractor.write_to_files(result)
        
        # Generate title from categories
        title_parts = []
        for category, secrets in sorted(result.categories.items()):
            if secrets:
                # Clean up category name for display
                display_name = category.replace('_', ' ')
                title_parts.append(f"{len(secrets)}x {display_name}")
        
        title = ", ".join(title_parts[:5])  # Max 5 categories in title
        
        # Build summary header
        header = build_credential_header(result)
        final_content = header + content
        
        # Post to SkyBin
        success = await post_to_skybin(final_content, title)
        
        return success
        
    except Exception as e:
        logger.error(f"Error processing file {filepath}: {e}")
        return False
    finally:
        # ALWAYS delete temp file
        try:
            if os.path.exists(filepath):
                os.unlink(filepath)
                logger.debug(f"  Deleted temp file: {filepath}")
        except Exception as e:
            logger.warning(f"  Failed to delete temp file: {e}")
```

#### 2. `telegram-scraper/credential_extractor.py`

**Add streaming-specific service categories:**
```python
# Add to SECRET_PATTERNS dict
"Amazon_Credentials": {
    "Amazon_Session": re.compile(r'(?i)session-token=([A-Za-z0-9%_-]{100,})'),
    "Amazon_Email_Pass": re.compile(r'(?i)amazon[^\\n]*?([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\\.[a-zA-Z0-9-.]+)[:\\s]+([^\\s]{6,})'),
},

"Prime_Video_Credentials": {
    "Prime_Token": re.compile(r'(?i)prime[_-]?video[^\\n]*?token\\s*[:=]\\s*[\\'\"]?([A-Za-z0-9_-]{30,})[\\'\"]?'),
},

"Paramount_Credentials": {
    "Paramount_Token": re.compile(r'(?i)paramount[^\\n]*?token\\s*[:=]\\s*[\\'\"]?([A-Za-z0-9_-]{30,})[\\'\"]?'),
},

"AppleTV_Credentials": {
    "AppleTV_Token": re.compile(r'(?i)apple[_-]?tv[^\\n]*?token\\s*[:=]\\s*[\\'\"]?([A-Za-z0-9_-]{30,})[\\'\"]?'),
},
```

**Update CATEGORY_FILES mapping:**
```python
CATEGORY_FILES.update({
    "Amazon_Credentials": "Amazon_Credentials.txt",
    "Prime_Video_Credentials": "Prime_Video_Credentials.txt",
    "Paramount_Credentials": "Paramount_Credentials.txt",
    "AppleTV_Credentials": "AppleTV_Credentials.txt",
})
```

### Part 3: Configuration Updates

- MAX_ARCHIVE_SIZE → 12GB (12 * 1024 * 1024 * 1024)
- MAX_CONCURRENT_DOWNLOADS → 5
- DOWNLOAD_TIMEOUT → 3600 seconds
- ONLY extract password text files from archives (e.g., passwords.txt, logins.txt, combo.txt)

---

## Part 3: Implementation Checklist

### Phase 1: UI/HTMX Upgrade
- [ ] Add axum-htmx to Cargo.toml
- [ ] Create HTML partial endpoints in src/web/mod.rs
- [ ] Update static/index.html with HTMX + Alpine.js
- [ ] Add skeleton loaders and severity styling to styles.css
- [ ] Test infinite scroll and auto-refresh
- [ ] Build and deploy to VPS

### Phase 2: Telegram Scraper Title Fix
- [ ] Remove generate_auto_title() channel name inclusion
- [ ] Update post_to_skybin() to skip if no credentials
- [ ] Test with real messages

### Phase 3: Large File Support
- [ ] Increase MAX_ARCHIVE_SIZE to 15GB
- [ ] Implement stream_download_file()
- [ ] Add progress logging
- [ ] Test with large archives

### Phase 4: File Processing Pipeline
- [ ] Create process_downloaded_file() function
- [ ] Ensure temp files are ALWAYS deleted
- [ ] Add file queue with concurrent workers
- [ ] Test end-to-end pipeline

### Phase 5: Testing & Deployment
- [ ] Run unit tests
- [ ] Test on staging
- [ ] Deploy to production VPS
- [ ] Monitor for issues

---

## File Locations

| File | Purpose |
|------|---------|
| `static/index.html` | Main feed page |
| `static/styles.css` | Global styles |
| `src/web/mod.rs` | Axum web routes |
| `src/web/handlers.rs` | Route handlers |
| `telegram-scraper/scraper.py` | Telegram scraper service |
| `telegram-scraper/credential_extractor.py` | Credential extraction & categorization |
| `/opt/skybin/extracted_secrets/` | Output directory for categorized secrets |
| `/tmp/skybin_downloads/` | Temp directory for large file downloads |

---

## Commands Reference

```bash
# Build Rust project
cargo build --release

# Run tests
cargo test --lib --verbose

# Deploy to VPS
rsync -avz --exclude 'target' . user@vps:/opt/skybin/

# Restart services on VPS
sudo systemctl restart skybin
sudo systemctl restart telegram-scraper

# Check logs
journalctl -u skybin -f
journalctl -u telegram-scraper -f

# Build on VPS
cd /opt/skybin && cargo build --release
```

---

## Resume Instructions

If this session is interrupted, load this file and:

1. Check the **Implementation Checklist** above to see what's done
2. Read the **Current Progress** section below
3. Continue from the last incomplete task

---

## Current Progress

**Last Updated:** 2025-12-06 02:46 UTC

### Completed:
- [x] Created implementation plan
- [x] Created this documentation file

### In Progress:
- [ ] Phase 1: UI/HTMX Upgrade (STARTING)

### Next Steps:
1. Read `src/web/mod.rs` to understand current routing structure
2. Add axum-htmx dependency to Cargo.toml
3. Create HTML partial endpoints
4. Update index.html with HTMX

---

## Notes & Decisions

1. **Why HTMX over React/Vue?** - No build step, progressive enhancement, works with existing static files
2. **Why Alpine.js?** - Lightweight (15kb), works inline in HTML, perfect for modals/toggles
3. **Why streaming downloads?** - 15GB files can't fit in memory, must stream to disk
4. **Why credential-only posting?** - Reduces noise, only posts actual leaks not channel chatter
