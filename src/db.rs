use crate::models::{Comment, Paste, SearchFilters};
use rusqlite::{params, Connection, Result as SqlResult, Row};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    SqliteError(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// Scraper health status
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScraperHealth {
    pub source: String,
    pub last_run: i64,
    pub success_rate: i64, // percentage 0-100
    pub total_runs: i64,
    pub pastes_found: i64,
    pub status: String, // "healthy", "degraded", "failing", "stale"
}

/// Format a search query for FTS5
/// Escapes special characters and adds prefix matching
fn format_fts_query(query: &str) -> String {
    // Split into words and format each
    let words: Vec<String> = query
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|word| {
            // Escape quotes in the word
            let escaped = word.replace('"', "\"\"").replace(['*', '(', ')'], "");
            // Add wildcard for prefix matching
            format!("\"{}\"*", escaped)
        })
        .collect();

    if words.is_empty() {
        return "*".to_string();
    }

    // Join with OR for more permissive matching
    words.join(" OR ")
}

/// Database connection wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the specified path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        Ok(Database { conn })
    }

    /// Initialize the database schema from migrations
    pub fn init_schema(&mut self) -> Result<()> {
        self.conn.execute_batch(
            r#"
-- Main pastes table
CREATE TABLE IF NOT EXISTS pastes (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    source_id TEXT,
    title TEXT,
    author TEXT,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL UNIQUE,
    url TEXT,
    syntax TEXT DEFAULT 'plaintext',
    matched_patterns TEXT,
    is_sensitive INTEGER DEFAULT 0,
    high_value INTEGER DEFAULT 0,
    staff_badge TEXT DEFAULT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    view_count INTEGER DEFAULT 0
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_expires_at ON pastes(expires_at);
CREATE INDEX IF NOT EXISTS idx_content_hash ON pastes(content_hash);
CREATE INDEX IF NOT EXISTS idx_created_at ON pastes(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_source ON pastes(source);
CREATE INDEX IF NOT EXISTS idx_is_sensitive ON pastes(is_sensitive);

-- Full-text search virtual table using FTS5
CREATE VIRTUAL TABLE IF NOT EXISTS pastes_fts USING fts5(
    id UNINDEXED,
    title,
    content,
    content='pastes',
    content_rowid='rowid'
);

-- Trigger to keep FTS5 in sync: INSERT
CREATE TRIGGER IF NOT EXISTS pastes_fts_insert AFTER INSERT ON pastes BEGIN
    INSERT INTO pastes_fts(rowid, id, title, content)
    VALUES (new.rowid, new.id, new.title, new.content);
END;

-- Trigger to keep FTS5 in sync: UPDATE
CREATE TRIGGER IF NOT EXISTS pastes_fts_update AFTER UPDATE ON pastes BEGIN
    UPDATE pastes_fts SET title = new.title, content = new.content
    WHERE rowid = new.rowid;
END;

-- Trigger to keep FTS5 in sync: DELETE
CREATE TRIGGER IF NOT EXISTS pastes_fts_delete AFTER DELETE ON pastes BEGIN
    DELETE FROM pastes_fts WHERE rowid = old.rowid;
END;

-- Trigger to auto-delete expired pastes on each insert
CREATE TRIGGER IF NOT EXISTS auto_purge_expired AFTER INSERT ON pastes BEGIN
    DELETE FROM pastes WHERE expires_at < unixepoch();
END;

-- Trigger to enforce max pastes limit (FIFO)
CREATE TRIGGER IF NOT EXISTS enforce_max_pastes AFTER INSERT ON pastes
WHEN (SELECT COUNT(*) FROM pastes) > 10000
BEGIN
    DELETE FROM pastes WHERE rowid IN (
        SELECT rowid FROM pastes
        ORDER BY created_at ASC
        LIMIT ((SELECT COUNT(*) FROM pastes) - 10000)
    );
END;

-- Short ID mapping table
CREATE TABLE IF NOT EXISTS short_ids (
    short_id TEXT PRIMARY KEY,
    paste_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (paste_id) REFERENCES pastes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_short_paste_id ON short_ids(paste_id);

-- Metadata table
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Comments table (anonymous)
CREATE TABLE IF NOT EXISTS comments (
    id TEXT PRIMARY KEY,
    paste_id TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (paste_id) REFERENCES pastes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_comments_paste_id ON comments(paste_id);
CREATE INDEX IF NOT EXISTS idx_comments_created_at ON comments(created_at DESC);

-- Scraper stats table for tracking source health
CREATE TABLE IF NOT EXISTS scraper_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    success INTEGER DEFAULT 0,
    failure INTEGER DEFAULT 0,
    pastes_found INTEGER DEFAULT 0,
    timestamp INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_scraper_stats_source ON scraper_stats(source);
CREATE INDEX IF NOT EXISTS idx_scraper_stats_timestamp ON scraper_stats(timestamp DESC);

-- Activity logs (anonymized)
CREATE TABLE IF NOT EXISTS activity_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action TEXT NOT NULL,
    details TEXT,
    timestamp INTEGER NOT NULL
);

-- Seen secrets for per-secret deduplication
-- hash is SHA256 of (secret_type + secret_value)
CREATE TABLE IF NOT EXISTS seen_secrets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    secret_hash TEXT NOT NULL UNIQUE,
    secret_type TEXT NOT NULL,
    first_seen INTEGER NOT NULL,
    last_seen INTEGER NOT NULL,
    occurrence_count INTEGER DEFAULT 1,
    source TEXT
);

CREATE INDEX IF NOT EXISTS idx_seen_secrets_hash ON seen_secrets(secret_hash);
CREATE INDEX IF NOT EXISTS idx_seen_secrets_type ON seen_secrets(secret_type);
CREATE INDEX IF NOT EXISTS idx_seen_secrets_first_seen ON seen_secrets(first_seen);

CREATE INDEX IF NOT EXISTS idx_activity_logs_timestamp ON activity_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_activity_logs_action ON activity_logs(action);

-- Trigger to keep activity logs under 10000 entries
CREATE TRIGGER IF NOT EXISTS enforce_max_activity_logs AFTER INSERT ON activity_logs
WHEN (SELECT COUNT(*) FROM activity_logs) > 10000
BEGIN
    DELETE FROM activity_logs WHERE id IN (
        SELECT id FROM activity_logs
        ORDER BY timestamp ASC
        LIMIT ((SELECT COUNT(*) FROM activity_logs) - 10000)
    );
END;

INSERT OR REPLACE INTO metadata (key, value) VALUES ('schema_version', '004');
INSERT OR REPLACE INTO metadata (key, value) VALUES ('created_at', unixepoch());
"#,
        )?;
        Ok(())
    }

    /// Insert a new paste into the database
    pub fn insert_paste(&mut self, paste: &Paste) -> Result<()> {
        let patterns_json = if let Some(patterns) = &paste.matched_patterns {
            serde_json::to_string(patterns)?
        } else {
            String::new()
        };

        self.conn.execute(
            "INSERT INTO pastes (id, source, source_id, title, author, content, content_hash, 
             url, syntax, matched_patterns, is_sensitive, high_value, staff_badge, created_at, expires_at, view_count)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                paste.id,
                paste.source,
                paste.source_id,
                paste.title,
                paste.author,
                paste.content,
                paste.content_hash,
                paste.url,
                paste.syntax,
                if patterns_json.is_empty() {
                    None::<String>
                } else {
                    Some(patterns_json)
                },
                if paste.is_sensitive { 1 } else { 0 },
                if paste.high_value { 1 } else { 0 },
                paste.staff_badge,
                paste.created_at,
                paste.expires_at,
                paste.view_count,
            ],
        )?;
        Ok(())
    }

    /// Get a paste by ID
    pub fn get_paste(&self, id: &str) -> Result<Option<Paste>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, source_id, title, author, content, content_hash, url, 
             syntax, matched_patterns, is_sensitive, high_value, staff_badge, created_at, expires_at, view_count 
             FROM pastes WHERE id = ?",
        )?;

        let result = stmt.query_row(params![id], Self::row_to_paste);

        match result {
            Ok(paste) => Ok(Some(paste)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a paste by content hash (for deduplication check)
    pub fn get_paste_by_hash(&self, hash: &str) -> Result<Option<Paste>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, source_id, title, author, content, content_hash, url, 
             syntax, matched_patterns, is_sensitive, high_value, staff_badge, created_at, expires_at, view_count 
             FROM pastes WHERE content_hash = ?",
        )?;

        let result = stmt.query_row(params![hash], Self::row_to_paste);

        match result {
            Ok(paste) => Ok(Some(paste)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get recent pastes with pagination
    pub fn get_recent_pastes(&self, limit: usize) -> Result<Vec<Paste>> {
        self.get_recent_pastes_offset(limit, 0)
    }

    /// Get recent pastes with limit and offset for pagination
    pub fn get_recent_pastes_offset(&self, limit: usize, offset: usize) -> Result<Vec<Paste>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, source_id, title, author, content, content_hash, url, 
             syntax, matched_patterns, is_sensitive, high_value, staff_badge, created_at, expires_at, view_count 
             FROM pastes ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )?;

        let pastes = stmt
            .query_map(params![limit, offset], Self::row_to_paste)?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(pastes)
    }

    /// Get pastes filtered by source and/or sensitive flag
    pub fn get_filtered_pastes(
        &self,
        source: Option<&str>,
        sensitive: Option<bool>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Paste>> {
        let sql = match (source, sensitive) {
            (Some(_), Some(_)) =>
                "SELECT id, source, source_id, title, author, content, content_hash, url, 
                 syntax, matched_patterns, is_sensitive, high_value, staff_badge, created_at, expires_at, view_count 
                 FROM pastes WHERE source = ?1 AND is_sensitive = ?2 ORDER BY created_at DESC LIMIT ?3 OFFSET ?4"
            .to_string(),
            (Some(_), None) =>
                "SELECT id, source, source_id, title, author, content, content_hash, url, 
                 syntax, matched_patterns, is_sensitive, high_value, staff_badge, created_at, expires_at, view_count 
                 FROM pastes WHERE source = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
            .to_string(),
            (None, Some(_)) =>
                "SELECT id, source, source_id, title, author, content, content_hash, url, 
                 syntax, matched_patterns, is_sensitive, high_value, staff_badge, created_at, expires_at, view_count 
                 FROM pastes WHERE is_sensitive = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
            .to_string(),
            (None, None) => return self.get_recent_pastes_offset(limit, offset),
        };

        let mut stmt = self.conn.prepare(&sql)?;

        let pastes = match (source, sensitive) {
            (Some(src), Some(sens)) => {
                let sens_int = if sens { 1 } else { 0 };
                stmt.query_map(params![src, sens_int, limit, offset], Self::row_to_paste)?
                    .collect::<SqlResult<Vec<_>>>()?
            }
            (Some(src), None) => stmt
                .query_map(params![src, limit, offset], Self::row_to_paste)?
                .collect::<SqlResult<Vec<_>>>()?,
            (None, Some(sens)) => {
                let sens_int = if sens { 1 } else { 0 };
                stmt.query_map(params![sens_int, limit, offset], Self::row_to_paste)?
                    .collect::<SqlResult<Vec<_>>>()?
            }
            (None, None) => unreachable!(),
        };

        Ok(pastes)
    }

    /// Get interesting pastes (with high-value pattern matches like API keys, tokens, SSH keys)
    /// Excludes false positives like generic credit cards and AWS account IDs
    pub fn get_interesting_pastes(&self, limit: usize, offset: usize) -> Result<Vec<Paste>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, source_id, title, author, content, content_hash, url, 
             syntax, matched_patterns, is_sensitive, high_value, staff_badge, created_at, expires_at, view_count 
             FROM pastes 
             WHERE is_sensitive = 1 
             AND matched_patterns IS NOT NULL
             AND matched_patterns NOT LIKE '%Credit Card%'
             AND (
                matched_patterns LIKE '%AWS Access Key%'
                OR matched_patterns LIKE '%GitHub Token%'
                OR matched_patterns LIKE '%Stripe%'
                OR matched_patterns LIKE '%Private Key%'
                OR matched_patterns LIKE '%Discord%'
                OR matched_patterns LIKE '%Telegram%'
                OR matched_patterns LIKE '%Database Connection%'
                OR matched_patterns LIKE '%JWT%'
                OR matched_patterns LIKE '%Bearer%'
                OR matched_patterns LIKE '%API Key%'
                OR matched_patterns LIKE '%Webhook%'
                OR matched_patterns LIKE '%OAuth%'
                OR matched_patterns LIKE '%Password%'
                OR matched_patterns LIKE '%Secret%'
             )
             ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )?;

        let pastes = stmt
            .query_map(params![limit, offset], Self::row_to_paste)?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(pastes)
    }

    /// Search pastes using full-text search
    pub fn search_pastes(&self, filters: &SearchFilters) -> Result<Vec<Paste>> {
        let raw_query = filters.query.as_deref().unwrap_or("").trim();
        let limit = filters.limit.unwrap_or(25).min(100);
        let offset = filters.offset.unwrap_or(0);

        // If no query, return recent pastes instead
        if raw_query.is_empty() {
            return self.get_recent_pastes_offset(limit, offset);
        }

        // Format query for FTS5 - escape special characters and add wildcards
        // FTS5 treats these as special: AND OR NOT ( ) " * ^
        let fts_query = format_fts_query(raw_query);

        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.source, p.source_id, p.title, p.author, p.content, p.content_hash, 
             p.url, p.syntax, p.matched_patterns, p.is_sensitive, p.created_at, p.expires_at, p.view_count
             FROM pastes p
             JOIN pastes_fts fts ON p.id = fts.id
             WHERE fts.pastes_fts MATCH ?
             AND (? IS NULL OR p.source = ?)
             AND (? IS NULL OR p.is_sensitive = ?)
             ORDER BY p.created_at DESC
             LIMIT ? OFFSET ?",
        )?;

        let is_sensitive = filters.is_sensitive.map(|v| if v { 1 } else { 0 });

        let pastes = stmt
            .query_map(
                params![
                    fts_query,
                    filters.source.as_ref(),
                    filters.source.as_ref(),
                    is_sensitive,
                    is_sensitive,
                    limit,
                    offset,
                ],
                Self::row_to_paste,
            )?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(pastes)
    }

    /// Get paste count
    pub fn get_paste_count(&self) -> Result<i64> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM pastes")?;
        let count = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    /// Get count of sensitive pastes
    pub fn get_sensitive_paste_count(&self) -> Result<i64> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM pastes WHERE is_sensitive = 1")?;
        let count = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    /// Get paste count by source
    pub fn get_paste_count_by_source(&self, source: &str) -> Result<i64> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM pastes WHERE source = ?")?;
        let count = stmt.query_row(params![source], |row| row.get(0))?;
        Ok(count)
    }

    /// Increment view count
    pub fn increment_view_count(&mut self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE pastes SET view_count = view_count + 1 WHERE id = ?",
            params![id],
        )?;
        Ok(())
    }

    /// Delete expired pastes manually (in addition to trigger)
    pub fn delete_expired_pastes(&mut self) -> Result<usize> {
        let changes = self
            .conn
            .execute("DELETE FROM pastes WHERE expires_at < unixepoch()", [])?;
        Ok(changes)
    }

    /// Insert a comment (with optional parent for replies)
    pub fn insert_comment(&mut self, comment: &Comment) -> Result<()> {
        self.conn.execute(
            "INSERT INTO comments (id, paste_id, parent_id, content, created_at) VALUES (?, ?, ?, ?, ?)",
            params![comment.id, comment.paste_id, comment.parent_id, comment.content, comment.created_at],
        )?;
        Ok(())
    }

    /// Get comments for a paste (including replies)
    pub fn get_comments(&self, paste_id: &str) -> Result<Vec<Comment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, paste_id, parent_id, content, created_at FROM comments WHERE paste_id = ? ORDER BY created_at ASC",
        )?;
        let comments = stmt
            .query_map(params![paste_id], |row| {
                Ok(Comment {
                    id: row.get(0)?,
                    paste_id: row.get(1)?,
                    parent_id: row.get(2)?,
                    content: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(comments)
    }

    // === ADMIN METHODS ===

    /// Delete a paste by ID (admin only)
    pub fn delete_paste(&mut self, id: &str) -> Result<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM pastes WHERE id = ?", params![id])?;
        Ok(changes > 0)
    }

    /// Delete a comment by ID (admin only)
    pub fn delete_comment(&mut self, id: &str) -> Result<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM comments WHERE id = ?", params![id])?;
        Ok(changes > 0)
    }

    /// Get all pastes (admin - paginated)
    pub fn get_all_pastes(&self, limit: usize, offset: usize) -> Result<Vec<Paste>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, source_id, title, author, content, content_hash, url, 
             syntax, matched_patterns, is_sensitive, high_value, staff_badge, created_at, expires_at, view_count 
             FROM pastes ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )?;
        let pastes = stmt
            .query_map(params![limit, offset], Self::row_to_paste)?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(pastes)
    }

    /// Get database stats (admin)
    pub fn get_db_stats(&self) -> Result<(i64, i64, i64)> {
        let paste_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM pastes", [], |r| r.get(0))?;
        let comment_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM comments", [], |r| r.get(0))?;
        let sensitive_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pastes WHERE is_sensitive = 1",
            [],
            |r| r.get(0),
        )?;
        Ok((paste_count, comment_count, sensitive_count))
    }

    /// Purge all pastes from a source (admin)
    pub fn purge_source(&mut self, source: &str) -> Result<usize> {
        let changes = self
            .conn
            .execute("DELETE FROM pastes WHERE source = ?", params![source])?;
        Ok(changes)
    }

    /// Get comment count for a paste
    pub fn get_comment_count(&self, paste_id: &str) -> Result<i64> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM comments WHERE paste_id = ?")?;
        let count = stmt.query_row(params![paste_id], |row| row.get(0))?;
        Ok(count)
    }

    /// Check if content hash exists (for deduplication)
    pub fn hash_exists(&self, hash: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM pastes WHERE content_hash = ? LIMIT 1")?;
        let exists = stmt.exists(params![hash])?;
        Ok(exists)
    }

    // === ANALYTICS METHODS ===

    /// Log a scraper run result
    pub fn log_scraper_stat(
        &mut self,
        source: &str,
        success: bool,
        pastes_found: usize,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT INTO scraper_stats (source, success, failure, pastes_found, timestamp) VALUES (?, ?, ?, ?, ?)",
            params![source, if success { 1 } else { 0 }, if success { 0 } else { 1 }, pastes_found as i64, now],
        )?;
        Ok(())
    }

    /// Get scraper stats for a time period (last N hours)
    pub fn get_scraper_stats(&self, hours: i64) -> Result<Vec<(String, i64, i64, i64)>> {
        let cutoff = chrono::Utc::now().timestamp() - (hours * 3600);
        let mut stmt = self.conn.prepare(
            "SELECT source, SUM(success), SUM(failure), SUM(pastes_found) 
             FROM scraper_stats WHERE timestamp > ? GROUP BY source",
        )?;
        let stats = stmt
            .query_map(params![cutoff], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(stats)
    }

    /// Get hourly scrape rates for charts (last 24 hours)
    pub fn get_hourly_scrape_rates(&self) -> Result<Vec<(i64, i64)>> {
        let cutoff = chrono::Utc::now().timestamp() - (24 * 3600);
        let mut stmt = self.conn.prepare(
            "SELECT (timestamp / 3600) * 3600 as hour, SUM(pastes_found) 
             FROM scraper_stats WHERE timestamp > ? GROUP BY hour ORDER BY hour",
        )?;
        let rates = stmt
            .query_map(params![cutoff], |row| Ok((row.get(0)?, row.get(1)?)))
            .map(|iter| iter.collect::<SqlResult<Vec<_>>>())??;
        Ok(rates)
    }

    /// Get pattern hit counts for charts
    pub fn get_pattern_hits(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT matched_patterns, COUNT(*) FROM pastes 
             WHERE matched_patterns IS NOT NULL AND matched_patterns != '' 
             GROUP BY matched_patterns ORDER BY COUNT(*) DESC LIMIT 20",
        )?;
        let hits = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get(1)?)))
            .map(|iter| iter.collect::<SqlResult<Vec<_>>>())??;
        Ok(hits)
    }

    /// Log an anonymized activity
    pub fn log_activity(&mut self, action: &str, details: Option<&str>) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT INTO activity_logs (action, details, timestamp) VALUES (?, ?, ?)",
            params![action, details, now],
        )?;
        Ok(())
    }

    /// Get recent activity logs
    pub fn get_activity_logs(&self, limit: usize) -> Result<Vec<(String, Option<String>, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT action, details, timestamp FROM activity_logs ORDER BY timestamp DESC LIMIT ?",
        )?;
        let logs = stmt
            .query_map(params![limit], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map(|iter| iter.collect::<SqlResult<Vec<_>>>())??;
        Ok(logs)
    }

    /// Get activity counts by action type (last 24 hours)
    pub fn get_activity_counts(&self) -> Result<Vec<(String, i64)>> {
        let cutoff = chrono::Utc::now().timestamp() - (24 * 3600);
        let mut stmt = self.conn.prepare(
            "SELECT action, COUNT(*) FROM activity_logs WHERE timestamp > ? GROUP BY action",
        )?;
        let counts = stmt
            .query_map(params![cutoff], |row| Ok((row.get(0)?, row.get(1)?)))
            .map(|iter| iter.collect::<SqlResult<Vec<_>>>())??;
        Ok(counts)
    }

    /// Get pastes per source (for source breakdown)
    pub fn get_source_breakdown(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT source, COUNT(*) FROM pastes GROUP BY source ORDER BY COUNT(*) DESC",
        )?;
        let breakdown = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map(|iter| iter.collect::<SqlResult<Vec<_>>>())??;
        Ok(breakdown)
    }

    /// Get comprehensive scraper health status
    /// Returns: (source, last_run, success_rate, total_runs, pastes_found, status)
    pub fn get_scraper_health(&self) -> Result<Vec<ScraperHealth>> {
        // Get stats for last hour for recency, and last 24h for health metrics
        let hour_ago = chrono::Utc::now().timestamp() - 3600;
        let day_ago = chrono::Utc::now().timestamp() - (24 * 3600);

        let mut stmt = self.conn.prepare(
            "SELECT 
                source,
                MAX(timestamp) as last_run,
                SUM(success) as successes,
                SUM(failure) as failures,
                SUM(pastes_found) as total_pastes
             FROM scraper_stats 
             WHERE timestamp > ?
             GROUP BY source
             ORDER BY source",
        )?;

        let health = stmt
            .query_map(params![day_ago], |row| {
                let source: String = row.get(0)?;
                let last_run: i64 = row.get(1)?;
                let successes: i64 = row.get(2)?;
                let failures: i64 = row.get(3)?;
                let pastes_found: i64 = row.get(4)?;

                let total = successes + failures;
                let success_rate = if total > 0 {
                    (successes as f64 / total as f64 * 100.0) as i64
                } else {
                    0
                };

                // Determine status based on metrics
                let status = if last_run < hour_ago {
                    "stale".to_string() // No runs in last hour
                } else if success_rate < 50 {
                    "degraded".to_string() // High failure rate
                } else if failures > 0 && successes == 0 {
                    "failing".to_string() // All recent runs failed
                } else {
                    "healthy".to_string()
                };

                Ok(ScraperHealth {
                    source,
                    last_run,
                    success_rate,
                    total_runs: total,
                    pastes_found,
                    status,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(health)
    }

    /// Helper function to convert a database row to a Paste struct
    fn row_to_paste(row: &Row) -> rusqlite::Result<Paste> {
        let matched_patterns_str: Option<String> = row.get(9)?;
        let matched_patterns: Option<Vec<crate::models::PatternMatch>> =
            if let Some(s) = &matched_patterns_str {
                if s.is_empty() {
                    None
                } else {
                    serde_json::from_str(s).ok()
                }
            } else {
                None
            };

        let is_sensitive: i32 = row.get(10)?;
        let high_value: i32 = row.get(11)?;
        let staff_badge: Option<String> = row.get(12)?;

        Ok(Paste {
            id: row.get(0)?,
            source: row.get(1)?,
            source_id: row.get(2)?,
            title: row.get(3)?,
            author: row.get(4)?,
            content: row.get(5)?,
            content_hash: row.get(6)?,
            url: row.get(7)?,
            syntax: row.get(8)?,
            matched_patterns,
            is_sensitive: is_sensitive != 0,
            high_value: high_value != 0,
            staff_badge,
            created_at: row.get(13)?,
            expires_at: row.get(14)?,
            view_count: row.get(15)?,
        })
    }

    /// Get high-value pastes (critical severity patterns: private keys, AWS keys, etc.)
    pub fn get_high_value_pastes(&self, limit: usize, offset: usize) -> Result<Vec<Paste>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, source_id, title, author, content, content_hash, url, 
             syntax, matched_patterns, is_sensitive, high_value, staff_badge, created_at, expires_at, view_count 
             FROM pastes 
             WHERE matched_patterns IS NOT NULL
             AND matched_patterns LIKE '%\"critical\"%'
             ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )?;

        let pastes = stmt
            .query_map(params![limit, offset], Self::row_to_paste)?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(pastes)
    }

    // === BULK DELETE METHODS ===

    /// Delete ALL pastes (admin only - use with extreme caution)
    pub fn delete_all_pastes(&mut self) -> Result<usize> {
        let count = self.conn.execute("DELETE FROM pastes", [])?;
        Ok(count)
    }

    /// Batch delete pastes by IDs (admin only)
    pub fn delete_pastes_by_ids(&mut self, ids: &[String]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }

        // Use a transaction for batch operations
        let tx = self.conn.transaction()?;
        let mut total_deleted = 0;

        for id in ids {
            let changes = tx.execute("DELETE FROM pastes WHERE id = ?", params![id])?;
            total_deleted += changes;
        }

        tx.commit()?;
        Ok(total_deleted)
    }

    /// Delete pastes older than N days (admin only)
    pub fn delete_pastes_older_than(&mut self, days: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - (days * 24 * 60 * 60);
        let count = self
            .conn
            .execute("DELETE FROM pastes WHERE created_at < ?", params![cutoff])?;
        Ok(count)
    }

    /// Get list of all unique sources in the database
    pub fn get_all_sources(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT source, COUNT(*) as count FROM pastes GROUP BY source ORDER BY count DESC",
        )?;
        let sources = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map(|iter| iter.collect::<SqlResult<Vec<_>>>())??;
        Ok(sources)
    }

    /// Delete pastes by search query (admin only)
    pub fn delete_pastes_by_search(&mut self, query: &str) -> Result<usize> {
        let fts_query = format_fts_query(query);
        let count = self.conn.execute(
            "DELETE FROM pastes WHERE rowid IN (
                SELECT rowid FROM pastes_fts WHERE pastes_fts MATCH ?
            )",
            params![fts_query],
        )?;
        Ok(count)
    }

    /// Check if a content hash already exists (for deduplication)
    pub fn check_hash_exists(&self, hash: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM pastes WHERE content_hash = ? LIMIT 1")?;
        let exists = stmt.exists(params![hash])?;
        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_paste() -> Paste {
        let now = Utc::now().timestamp();
        let uuid = Uuid::new_v4().to_string();
        Paste {
            id: uuid.clone(),
            source: "test_source".to_string(),
            source_id: Some("source_123".to_string()),
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            content: "Test content here".to_string(),
            content_hash: uuid, // Use UUID as hash to ensure uniqueness
            url: Some("https://example.com".to_string()),
            syntax: "plaintext".to_string(),
            matched_patterns: None,
            is_sensitive: false,
            high_value: false,
            staff_badge: None,
            created_at: now,
            expires_at: now + (7 * 24 * 60 * 60),
            view_count: 0,
        }
    }

    #[test]
    fn test_database_init() {
        let _db = Database::open(":memory:").unwrap();
        // If initialization succeeds without panicking, test passes
        assert!(true);
    }

    #[test]
    fn test_insert_and_retrieve_paste() {
        let mut db = Database::open(":memory:").unwrap();
        db.init_schema().unwrap();

        let paste = create_test_paste();
        let paste_id = paste.id.clone();

        db.insert_paste(&paste).unwrap();
        let retrieved = db.get_paste(&paste_id).unwrap();

        assert!(retrieved.is_some());
        let retrieved_paste = retrieved.unwrap();
        assert_eq!(retrieved_paste.id, paste.id);
        assert_eq!(retrieved_paste.content, "Test content here");
    }

    #[test]
    fn test_get_recent_pastes() {
        let mut db = Database::open(":memory:").unwrap();
        db.init_schema().unwrap();

        for i in 0..3 {
            let mut paste = create_test_paste();
            paste.id = format!("paste_{}", i);
            db.insert_paste(&paste).unwrap();
        }

        let recent = db.get_recent_pastes(10).unwrap();
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_get_paste_count() {
        let mut db = Database::open(":memory:").unwrap();
        db.init_schema().unwrap();

        assert_eq!(db.get_paste_count().unwrap(), 0);

        for i in 0..5 {
            let mut paste = create_test_paste();
            paste.id = format!("paste_{}", i);
            db.insert_paste(&paste).unwrap();
        }

        assert_eq!(db.get_paste_count().unwrap(), 5);
    }

    #[test]
    fn test_search_filters() {
        let mut db = Database::open(":memory:").unwrap();
        db.init_schema().unwrap();

        let mut paste1 = create_test_paste();
        paste1.id = "paste1".to_string();
        paste1.source = "pastebin".to_string();
        db.insert_paste(&paste1).unwrap();

        let mut paste2 = create_test_paste();
        paste2.id = "paste2".to_string();
        paste2.source = "gists".to_string();
        db.insert_paste(&paste2).unwrap();

        let count = db.get_paste_count_by_source("pastebin").unwrap();
        assert_eq!(count, 1);
    }
}
