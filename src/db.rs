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

INSERT OR REPLACE INTO metadata (key, value) VALUES ('schema_version', '002');
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
             url, syntax, matched_patterns, is_sensitive, created_at, expires_at, view_count)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
             syntax, matched_patterns, is_sensitive, created_at, expires_at, view_count 
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
             syntax, matched_patterns, is_sensitive, created_at, expires_at, view_count 
             FROM pastes WHERE content_hash = ?",
        )?;

        let result = stmt.query_row(params![hash], Self::row_to_paste);

        match result {
            Ok(paste) => Ok(Some(paste)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get recent pastes
    pub fn get_recent_pastes(&self, limit: usize) -> Result<Vec<Paste>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, source_id, title, author, content, content_hash, url, 
             syntax, matched_patterns, is_sensitive, created_at, expires_at, view_count 
             FROM pastes ORDER BY created_at DESC LIMIT ?",
        )?;

        let pastes = stmt
            .query_map(params![limit], Self::row_to_paste)?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(pastes)
    }

    /// Search pastes using full-text search
    pub fn search_pastes(&self, filters: &SearchFilters) -> Result<Vec<Paste>> {
        let query = filters.query.as_deref().unwrap_or("*");
        let limit = filters.limit.unwrap_or(10).min(100);
        let offset = filters.offset.unwrap_or(0);

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
                    query,
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

    /// Insert a comment
    pub fn insert_comment(&mut self, comment: &Comment) -> Result<()> {
        self.conn.execute(
            "INSERT INTO comments (id, paste_id, content, created_at) VALUES (?, ?, ?, ?)",
            params![comment.id, comment.paste_id, comment.content, comment.created_at],
        )?;
        Ok(())
    }

    /// Get comments for a paste
    pub fn get_comments(&self, paste_id: &str) -> Result<Vec<Comment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, paste_id, content, created_at FROM comments WHERE paste_id = ? ORDER BY created_at ASC",
        )?;
        let comments = stmt
            .query_map(params![paste_id], |row| {
                Ok(Comment {
                    id: row.get(0)?,
                    paste_id: row.get(1)?,
                    content: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(comments)
    }

    /// Get comment count for a paste
    pub fn get_comment_count(&self, paste_id: &str) -> Result<i64> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM comments WHERE paste_id = ?")?;
        let count = stmt.query_row(params![paste_id], |row| row.get(0))?;
        Ok(count)
    }

    /// Check if content hash exists (for deduplication)
    pub fn hash_exists(&self, hash: &str) -> Result<bool> {
        let mut stmt = self.conn.prepare("SELECT 1 FROM pastes WHERE content_hash = ? LIMIT 1")?;
        let exists = stmt.exists(params![hash])?;
        Ok(exists)
    }

    /// Helper function to convert a database row to a Paste struct
    fn row_to_paste(row: &Row) -> rusqlite::Result<Paste> {
        let matched_patterns_str: Option<String> = row.get(9)?;
        let matched_patterns = if let Some(s) = matched_patterns_str {
            if s.is_empty() {
                None
            } else {
                serde_json::from_str(&s).ok()
            }
        } else {
            None
        };

        let is_sensitive: i32 = row.get(10)?;

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
            created_at: row.get(11)?,
            expires_at: row.get(12)?,
            view_count: row.get(13)?,
        })
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
