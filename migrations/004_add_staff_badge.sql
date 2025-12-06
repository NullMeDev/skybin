-- Migration 004: Add staff badge support
-- Run with: sqlite3 skybin.db < migrations/004_add_staff_badge.sql

-- Add staff_badge column (NULL for non-staff posts)
ALTER TABLE pastes ADD COLUMN staff_badge TEXT DEFAULT NULL;

-- Update schema version
UPDATE metadata SET value = '004' WHERE key = 'schema_version';
