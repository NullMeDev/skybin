-- SkyBin Database Schema
-- Version: 001_initial

-- Main pastes table
CREATE TABLE IF NOT EXISTS pastes (
    id TEXT PRIMARY KEY,                  -- UUID v4
    source TEXT NOT NULL,                 -- 'pastebin', 'gist', 'user_upload', etc.
    source_id TEXT,                       -- Original ID from the source
    title TEXT,                           -- Paste title
    author TEXT,                          -- Author/uploader name
    content TEXT NOT NULL,                -- Full paste content
    content_hash TEXT NOT NULL UNIQUE,    -- SHA256 hash for deduplication
    url TEXT,                             -- Original URL (if scraped)
    syntax TEXT DEFAULT 'plaintext',      -- Language/syntax type
    matched_patterns TEXT,                -- JSON array of detected patterns
    is_sensitive INTEGER DEFAULT 0,       -- Boolean: contains sensitive data
    created_at INTEGER NOT NULL,          -- Unix timestamp (UTC)
    expires_at INTEGER NOT NULL,          -- Unix timestamp for auto-purge
    view_count INTEGER DEFAULT 0          -- View counter
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

-- Short ID mapping table for user-friendly URLs
CREATE TABLE IF NOT EXISTS short_ids (
    short_id TEXT PRIMARY KEY,  -- base62 short ID
    paste_id TEXT NOT NULL,     -- UUID reference to pastes.id
    created_at INTEGER NOT NULL,
    FOREIGN KEY (paste_id) REFERENCES pastes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_short_paste_id ON short_ids(paste_id);

-- Metadata table to track schema version
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR REPLACE INTO metadata (key, value) VALUES ('schema_version', '001');
INSERT OR REPLACE INTO metadata (key, value) VALUES ('created_at', unixepoch());
