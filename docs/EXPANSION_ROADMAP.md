# SkyBin Expansion & Improvement Roadmap

**Generated:** December 6, 2025  
**Status:** Planning Phase

---

## ✅ COMPLETED: Telegram Scraper Improvements

- **Service-based credential classification** - 50+ service patterns (Gmail, Roblox, Netflix, etc.)
- **Smart title generation** - "5x Gmail Logins", "3x Roblox, 2x Steam Logins", or "40x Assorted Logins"
- **Summary headers** - Total credentials, breakdown by service (top 10)
- **Lowered thresholds** - Post any content with ≥1 credential
- **Removed channel prefix** - Titles focus on data, not source
- **Dedup on raw content** - Header doesn't affect duplicate detection

---

## 📊 12+ EXPANSION STRATEGIES

### **Paste Site Expansion (6 sources)**

#### 1. Pastery.net
**Effort:** Low | **Value:** Medium
```rust
// src/scrapers/pastery.rs
pub struct PasteryScraper {
    base_url: String,
}

impl Scraper for PasteryScraper {
    async fn fetch_recent(&self, client: &Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        // API: https://www.pastery.net/api/
        // Endpoint: GET /api/paste/?duration=day
        // Returns JSON array of recent pastes
    }
}
```

#### 2. Nekobin.com
**Effort:** Low | **Value:** Medium
- JSON API: `https://nekobin.com/api/documents`
- Popular in Asian dev communities
- Often contains gaming account dumps

#### 3. CodeShare.io
**Effort:** Medium | **Value:** Medium
- Real-time code collaboration
- Public rooms endpoint: `/api/rooms/public`
- May contain leaked API configs

#### 4. Paste.Mozilla.org
**Effort:** Medium | **Value:** High
- Mozilla's official paste service
- Internal dev notes, Firefox configs
- API: Similar to Hastebin

#### 5. Hastebin Network
**Effort:** Low | **Value:** Medium
- Multiple instances:
  - `toptal.com/developers/hastebin`
  - `paste.gg`
  - `bin.disroot.org`
- Unified scraper for Hastebin-compatible APIs

#### 6. Rentry.co
**Effort:** Low | **Value:** Medium
- Markdown paste site
- Often used for combo lists
- No public API - HTML scraping required

---

### **Rate Limiting & Infrastructure (4 strategies)**

#### 7. Rotating Proxy Pool
**Implementation:**
```toml
# config.toml
[proxies]
enabled = true
providers = [
    "socks5://user:pass@proxy1.example.com:1080",
    "http://proxy2.example.com:8080",
    "socks5://user:pass@proxy3.example.com:1080"
]
rotation_strategy = "round_robin" # or "random" or "least_used"
max_failures = 3 # Remove proxy after 3 consecutive failures
```

```rust
// src/proxy_manager.rs
pub struct ProxyManager {
    proxies: Vec<ProxyConfig>,
    failure_counts: HashMap<usize, usize>,
    current_index: AtomicUsize,
}

impl ProxyManager {
    pub fn next_proxy(&self) -> Option<&ProxyConfig> {
        // Round-robin with failure tracking
        // Skip proxies with failure_count >= max_failures
    }
}
```

#### 8. Adaptive Backoff System
**Per-source state tracking:**
```rust
// src/scheduler.rs
pub struct SourceState {
    source_name: String,
    last_success: Instant,
    consecutive_failures: u32,
    backoff_multiplier: f64, // Starts at 1.0, increases exponentially
    is_rate_limited: bool,
}

impl SourceState {
    pub fn compute_next_delay(&self, base_interval: Duration) -> Duration {
        if self.is_rate_limited {
            // Aggressive backoff for rate-limited sources
            base_interval * (2u32.pow(self.consecutive_failures)) * 2
        } else {
            base_interval * (self.backoff_multiplier as u32)
        }
    }
}
```

#### 9. Request Queue with Priority
**High-value sources get priority:**
```rust
pub enum SourcePriority {
    Critical, // Telegram, BreachForums RSS
    High,     // GitHub Gists, Pastebin
    Medium,   // Most paste sites
    Low,      // Low-yield or stale sources
}

pub struct PriorityQueue {
    critical: VecDeque<ScraperTask>,
    high: VecDeque<ScraperTask>,
    medium: VecDeque<ScraperTask>,
    low: VecDeque<ScraperTask>,
}

impl PriorityQueue {
    pub fn pop(&mut self) -> Option<ScraperTask> {
        self.critical.pop_front()
            .or_else(|| self.high.pop_front())
            .or_else(|| self.medium.pop_front())
            .or_else(|| self.low.pop_front())
    }
}
```

#### 10. Distributed Scraping
**Run multiple instances, POST to central API:**
```bash
# VPS 1 (US East)
./skybin-scraper --config us-east.toml --central-api https://skybin.lol/api/paste

# VPS 2 (EU West)
./skybin-scraper --config eu-west.toml --central-api https://skybin.lol/api/paste

# VPS 3 (Asia Pacific)
./skybin-scraper --config asia.toml --central-api https://skybin.lol/api/paste
```

Each instance focuses on different sources to avoid overlap.

---

### **Telegram Channel Discovery (3 strategies)**

#### 11. Auto-Discovery from Pastes
**Scan for invite links:**
```rust
// src/patterns/telegram.rs
lazy_static! {
    static ref TELEGRAM_INVITE: Regex = Regex::new(
        r"(?:t\.me/|telegram\.me/|telegram\.dog/)(?:joinchat/)?([a-zA-Z0-9_-]+)"
    ).unwrap();
}

pub fn extract_telegram_invites(content: &str) -> Vec<String> {
    TELEGRAM_INVITE.captures_iter(content)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
        .collect()
}
```

**Admin approval workflow:**
1. Extract invites from posted pastes
2. Store in `pending_invites` table
3. Admin reviews in `/x` panel
4. Approve → Telegram scraper auto-joins

#### 12. Channel Aggregator APIs
**Use telemetr.io or combot.org:**
```rust
// Scrape telemetr.io trending channels
// Filter by keywords: "logs", "combo", "leak", "dump", "stealer"
let trending_url = "https://telemetr.io/en/channels?search=logs&sort=members";
```

#### 13. Cross-Channel Analysis
**Track similar content across channels:**
```sql
-- Find channels posting similar content
SELECT 
    c1.channel_name AS channel_a,
    c2.channel_name AS channel_b,
    COUNT(*) AS shared_hashes
FROM channel_posts c1
JOIN channel_posts c2 ON c1.content_hash = c2.content_hash
WHERE c1.channel_name != c2.channel_name
GROUP BY channel_a, channel_b
HAVING shared_hashes >= 5
ORDER BY shared_hashes DESC;
```

**Use similarity clustering to discover related channels.**

---

### **Alternative Data Sources (5 sources)**

#### 14. Discord Invite Scanner
**Requires Discord bot or selfbot:**
```rust
// Join public servers from invite links
// Monitor #leaks, #dumps, #combos channels
// Extract and post to SkyBin

// Note: Against Discord TOS for selfbots
// Use official bot with read message history permission
```

#### 15. GitHub Code Search Integration
**Already in roadmap - HIGH PRIORITY:**
```rust
// src/scrapers/github_code.rs
pub async fn search_exposed_secrets(
    client: &Client,
    token: &str,
) -> Vec<DiscoveredPaste> {
    let queries = [
        "password filename:.env",
        "api_key filename:config",
        "AWS_SECRET_ACCESS_KEY",
        "mongodb+srv://",
    ];
    
    for query in queries {
        let url = format!(
            "https://api.github.com/search/code?q={}",
            urlencoding::encode(query)
        );
        // Parse results, fetch raw files
    }
}
```

#### 16. Onion/Tor Paste Sites
**DarkPaste, ZeroBin on .onion:**
```toml
[tor]
enabled = true
socks_proxy = "socks5://127.0.0.1:9050" # Tor SOCKS proxy
circuits = 3 # Number of circuits to rotate
```

**Sites:**
- `http://darkpastexyz.onion`
- `http://zerobinxyz.onion`
- `http://strongboxonion3.onion`

#### 17. Pastebin Alternatives
**JustPaste.it, Ghostbin, Controlc:**
```rust
// Add to sources config
pub struct JustPasteItScraper;
pub struct GhostbinScraper;
pub struct ControlcScraper;
pub struct PrivateBinScraper; // For public instances
```

#### 18. RSS Feed Aggregation
**BreachForums, Exposed.lol:**
```rust
// src/scrapers/rss.rs
pub struct RssFeedScraper {
    feeds: Vec<String>,
}

// Feeds to monitor:
// - https://breachforums.st/rss (if available)
// - https://exposed.lol/feed
// - https://databases.today/feed
```

---

## 🎨 10+ UI/FRONTEND IMPROVEMENTS

### **Visual & UX Enhancements (5)**

#### 1. Dark/Light Theme Toggle
```javascript
// static/theme.js
function toggleTheme() {
    const current = document.documentElement.getAttribute('data-theme');
    const next = current === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    localStorage.setItem('theme', next);
}

// On load
const saved = localStorage.getItem('theme') || 'dark';
document.documentElement.setAttribute('data-theme', saved);
```

```css
/* static/styles.css */
:root[data-theme="light"] {
    --bg-primary: #ffffff;
    --bg-secondary: #f5f5f5;
    --text-primary: #1a1a1a;
    --border: #e0e0e0;
}

:root[data-theme="dark"] {
    --bg-primary: #1a1a1a;
    --bg-secondary: #2a2a2a;
    --text-primary: #e0e0e0;
    --border: #3a3a3a;
}
```

#### 2. Paste Cards Redesign
**Add colored severity indicators:**
```html
<div class="paste-card" data-severity="critical">
    <div class="severity-bar"></div>
    <div class="paste-header">
        <div class="service-icons">
            <img src="/static/icons/gmail.svg" title="Gmail">
            <img src="/static/icons/roblox.svg" title="Roblox">
        </div>
        <span class="credential-badge">5 credentials</span>
    </div>
    <h3>5x Gmail, 3x Roblox Logins</h3>
    <div class="paste-meta">
        <span class="source">telegram</span>
        <span class="time">2 min ago</span>
    </div>
</div>
```

```css
.paste-card[data-severity="critical"] .severity-bar {
    background: linear-gradient(90deg, #dc2626 0%, transparent 100%);
    height: 100%;
    width: 4px;
    position: absolute;
    left: 0;
}

.credential-badge {
    background: var(--accent);
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 600;
}
```

#### 3. Infinite Scroll with Virtual Scrolling
```javascript
// Use Intersection Observer
const sentinel = document.querySelector('#load-more-sentinel');
const observer = new IntersectionObserver(entries => {
    if (entries[0].isIntersecting && !loading) {
        loadMorePastes();
    }
});
observer.observe(sentinel);
```

#### 4. Syntax Highlighting
```html
<!-- Add Prism.js -->
<script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/prism.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-python.min.js"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/themes/prism-tomorrow.min.css">
```

```javascript
// Auto-detect language and highlight
document.querySelectorAll('pre code').forEach(block => {
    Prism.highlightElement(block);
});
```

#### 5. Search Filters UI
```html
<div class="search-filters">
    <select id="source-filter">
        <option value="">All Sources</option>
        <option value="telegram">Telegram</option>
        <option value="pastebin">Pastebin</option>
        <option value="github">GitHub</option>
    </select>
    
    <select id="severity-filter">
        <option value="">All Severities</option>
        <option value="critical">Critical</option>
        <option value="high">High</option>
        <option value="medium">Medium</option>
    </select>
    
    <input type="date" id="date-from" placeholder="From">
    <input type="date" id="date-to" placeholder="To">
</div>
```

---

### **Interactive Features (3)**

#### 6. Copy-to-Clipboard Buttons
```javascript
// Add copy button to paste content
function addCopyButton(element) {
    const btn = document.createElement('button');
    btn.className = 'copy-btn';
    btn.innerHTML = '<svg>...</svg> Copy';
    btn.onclick = () => {
        navigator.clipboard.writeText(element.textContent);
        btn.innerHTML = '<svg>...</svg> Copied!';
        setTimeout(() => {
            btn.innerHTML = '<svg>...</svg> Copy';
        }, 2000);
    };
    element.parentNode.insertBefore(btn, element);
}
```

#### 7. Credential Extraction View
```html
<div class="paste-view">
    <div class="tabs">
        <button class="tab active" data-tab="raw">Raw</button>
        <button class="tab" data-tab="extracted">Extracted</button>
        <button class="tab" data-tab="stats">Stats</button>
    </div>
    
    <div class="tab-content" data-content="extracted">
        <table class="credentials-table">
            <thead>
                <tr>
                    <th>Service</th>
                    <th>Email/Username</th>
                    <th>Password</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody id="credentials-tbody">
                <!-- Populated via JS -->
            </tbody>
        </table>
    </div>
</div>
```

#### 8. Real-Time Notifications
```javascript
// Server-Sent Events for new pastes
const eventSource = new EventSource('/api/events');
eventSource.onmessage = (event) => {
    const paste = JSON.parse(event.data);
    if (paste.severity === 'critical' || paste.severity === 'high') {
        showToast(`New ${paste.severity} paste: ${paste.title}`, paste.id);
    }
};

function showToast(message, pasteId) {
    const toast = document.createElement('div');
    toast.className = 'toast';
    toast.innerHTML = `
        <span>${message}</span>
        <a href="/paste/${pasteId}">View</a>
    `;
    document.body.appendChild(toast);
    setTimeout(() => toast.remove(), 5000);
}
```

---

### **Performance & Polish (3)**

#### 9. Loading States with Skeleton Screens
```html
<div class="skeleton-paste">
    <div class="skeleton-bar"></div>
    <div class="skeleton-title"></div>
    <div class="skeleton-meta"></div>
</div>
```

```css
.skeleton-bar, .skeleton-title, .skeleton-meta {
    background: linear-gradient(
        90deg,
        var(--bg-secondary) 25%,
        var(--bg-tertiary) 50%,
        var(--bg-secondary) 75%
    );
    background-size: 200% 100%;
    animation: skeleton-loading 1.5s infinite;
}

@keyframes skeleton-loading {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
}
```

#### 10. Empty States
```html
<div class="empty-state">
    <img src="/static/illustrations/no-results.svg" alt="No results">
    <h3>No pastes found</h3>
    <p>Try adjusting your search filters or check back later.</p>
</div>
```

#### 11. Keyboard Shortcuts
```javascript
document.addEventListener('keydown', (e) => {
    // "/" - Focus search
    if (e.key === '/' && !isInputFocused()) {
        e.preventDefault();
        document.querySelector('#search-input').focus();
    }
    
    // "Esc" - Close modals
    if (e.key === 'Escape') {
        closeAllModals();
    }
    
    // "n/p" - Next/prev paste
    if (e.key === 'n') navigatePaste('next');
    if (e.key === 'p') navigatePaste('prev');
});
```

---

## 🛠️ ADMIN PANEL IMPROVEMENTS

### **Staff Badge System**

**Database schema addition:**
```sql
-- Add staff_posts table
CREATE TABLE IF NOT EXISTS staff_posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    paste_id TEXT NOT NULL UNIQUE,
    staff_name TEXT NOT NULL,
    staff_badge TEXT NOT NULL, -- "Owner", "Admin", "Moderator"
    posted_at INTEGER NOT NULL,
    FOREIGN KEY (paste_id) REFERENCES pastes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_staff_posts_paste_id ON staff_posts(paste_id);
```

**Admin panel UI:**
```html
<!-- templates/admin/create_paste.html -->
<form id="admin-paste-form">
    <label>
        <input type="checkbox" id="mark-as-staff" checked>
        Post as Staff Member
    </label>
    
    <select id="staff-badge">
        <option value="Owner">NullMeDev - SkyBin Owner</option>
        <option value="Admin">Admin</option>
        <option value="Moderator">Moderator</option>
    </select>
    
    <textarea id="paste-content" required></textarea>
    <input type="text" id="paste-title" placeholder="Title">
    
    <button type="submit">Create Paste</button>
</form>
```

**Display badge on pastes:**
```html
<!-- templates/paste_view.html -->
{{#if staff_post}}
<div class="staff-badge-container">
    <span class="staff-badge staff-badge-{{staff_post.badge_type}}">
        <svg class="badge-icon">...</svg>
        {{staff_post.staff_name}}
    </span>
    <span class="verified-checkmark">✓</span>
</div>
{{/if}}
```

```css
.staff-badge-owner {
    background: linear-gradient(135deg, #f59e0b 0%, #dc2626 100%);
    color: white;
    padding: 6px 12px;
    border-radius: 6px;
    font-weight: 700;
    box-shadow: 0 2px 8px rgba(245, 158, 11, 0.3);
}

.verified-checkmark {
    color: #10b981;
    font-size: 20px;
    margin-left: 8px;
}
```

---

### **Enhanced Admin Tools (10 features)**

#### 1. Bulk Operations
```html
<div class="bulk-actions">
    <input type="checkbox" id="select-all">
    <button onclick="bulkDelete()">Delete Selected</button>
    <button onclick="bulkMark('sensitive')">Mark Sensitive</button>
    <button onclick="bulkExport()">Export Selected</button>
</div>
```

#### 2. Advanced Analytics Dashboard
```javascript
// /api/x/analytics
{
    "pastes_per_day": [120, 145, 98, ...],
    "top_sources": {"telegram": 450, "pastebin": 120},
    "credential_types": {"email_pass": 340, "api_keys": 120},
    "trending_services": ["Gmail", "Roblox", "Netflix"],
    "detection_accuracy": 94.5
}
```

#### 3. Source Health Monitoring
```html
<div class="source-health">
    <div class="source-card" data-status="healthy">
        <h4>Telegram</h4>
        <span class="status-indicator"></span>
        <div class="stats">
            <span>450 pastes/day</span>
            <span>99.2% uptime</span>
        </div>
    </div>
</div>
```

#### 4. Pattern Management UI
```html
<div class="pattern-editor">
    <h3>Custom Patterns</h3>
    <button onclick="addPattern()">+ Add Pattern</button>
    
    <div class="pattern-list">
        <div class="pattern-item">
            <input type="text" value="custom_api_key_.*">
            <select>
                <option value="critical">Critical</option>
                <option value="high">High</option>
            </select>
            <button onclick="testPattern(this)">Test</button>
            <button onclick="deletePattern(this)">Delete</button>
        </div>
    </div>
</div>
```

#### 5. IP Blocklist Management
```html
<div class="blocklist-manager">
    <h3>IP Blocklist</h3>
    <input type="text" id="block-ip" placeholder="192.168.1.1">
    <button onclick="addToBlocklist()">Block</button>
    
    <table>
        <thead>
            <tr><th>IP</th><th>Reason</th><th>Blocked At</th><th>Actions</th></tr>
        </thead>
        <tbody id="blocklist-tbody"></tbody>
    </table>
</div>
```

#### 6. Real-Time Activity Feed
```html
<div class="activity-feed">
    <h3>Live Activity</h3>
    <div id="activity-stream">
        <div class="activity-item">
            <span class="activity-icon">📥</span>
            <span>New paste from telegram: "5x Gmail Logins"</span>
            <span class="activity-time">2s ago</span>
        </div>
    </div>
</div>
```

#### 7. Scheduled Tasks UI
```html
<div class="scheduled-tasks">
    <h3>Scheduled Tasks</h3>
    <button onclick="createTask()">+ New Task</button>
    
    <div class="task-list">
        <div class="task-card">
            <h4>Daily Cleanup</h4>
            <span>Delete pastes older than 7 days</span>
            <span class="cron">0 0 * * *</span>
            <button onclick="runNow(this)">Run Now</button>
        </div>
    </div>
</div>
```

#### 8. Webhook Configuration
```html
<div class="webhook-config">
    <h3>Outbound Webhooks</h3>
    <form>
        <input type="url" placeholder="https://discord.com/api/webhooks/...">
        <select multiple>
            <option value="critical">Critical Pastes</option>
            <option value="high">High Severity</option>
            <option value="staff">Staff Posts</option>
        </select>
        <button type="submit">Add Webhook</button>
    </form>
</div>
```

#### 9. API Key Management
```html
<div class="api-keys">
    <h3>API Keys</h3>
    <button onclick="generateKey()">Generate New Key</button>
    
    <table>
        <thead>
            <tr><th>Key</th><th>Permissions</th><th>Created</th><th>Actions</th></tr>
        </thead>
        <tbody id="api-keys-tbody"></tbody>
    </table>
</div>
```

#### 10. Audit Log Viewer
```html
<div class="audit-log">
    <h3>Audit Log</h3>
    <input type="search" placeholder="Search actions...">
    
    <table>
        <thead>
            <tr><th>Time</th><th>User</th><th>Action</th><th>Details</th></tr>
        </thead>
        <tbody>
            <tr>
                <td>2025-12-06 05:00</td>
                <td>NullMeDev</td>
                <td>DELETE_PASTE</td>
                <td>Paste ID: abc123</td>
            </tr>
        </tbody>
    </table>
</div>
```

---

## 🚀 IMPLEMENTATION PRIORITY

### **Phase 1 (Week 1) - Quick Wins**
1. ✅ Telegram title improvements (COMPLETED)
2. Staff badge system
3. Dark/Light theme toggle
4. Copy-to-clipboard buttons
5. Advanced admin analytics

### **Phase 2 (Week 2-3) - Expansion**
6. Pastery.net + Nekobin scrapers
7. Rotating proxy pool
8. Adaptive backoff system
9. Paste cards redesign
10. Search filters UI

### **Phase 3 (Week 4+) - Advanced**
11. GitHub Code Search integration
12. Tor paste site scrapers
13. Real-time SSE notifications
14. Credential extraction view
15. Distributed scraping setup

---

## 📌 NOTES

- All UI changes should maintain HTMX/Alpine.js architecture (no build step)
- Admin features require `AdminAuth` middleware
- Rate limiting strategies should be configurable via `config.toml`
- Staff badges require database migration before deployment
- Proxy rotation needs VPN/residential proxies (avoid datacenter IPs)

**End of Roadmap**
