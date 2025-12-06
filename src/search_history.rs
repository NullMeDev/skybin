use crate::models::SearchFilters;
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

/// Search history entry (per-session tracking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub query: String,
    pub timestamp: i64,
    pub result_count: usize,
    pub filters: SearchFilters,
}

/// Saved search with label (client-side persistence via localStorage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSearch {
    pub id: String,
    pub label: String,
    pub query: String,
    pub filters: SearchFilters,
    pub created_at: i64,
}

/// Search suggestions (pattern-based autocomplete)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub category: String, // "pattern", "source", "recent"
    pub count: Option<usize>,
}

pub struct SearchHistory {
    conn: Connection,
}

impl SearchHistory {
    /// Initialize search history database (in-memory or file-based)
    pub fn new(db_path: &str) -> SqlResult<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS search_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                result_count INTEGER NOT NULL,
                filters TEXT
            )",
            [],
        )?;

        // Create index separately
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON search_history(timestamp)",
            [],
        )?;

        // Auto-cleanup: keep only last 1000 searches
        conn.execute(
            "DELETE FROM search_history WHERE id NOT IN (
                SELECT id FROM search_history ORDER BY timestamp DESC LIMIT 1000
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Record a search query
    pub fn record_search(&mut self, entry: &SearchHistoryEntry) -> SqlResult<()> {
        let filters_json = serde_json::to_string(&entry.filters).unwrap_or_default();
        self.conn.execute(
            "INSERT INTO search_history (query, timestamp, result_count, filters) VALUES (?, ?, ?, ?)",
            params![entry.query, entry.timestamp, entry.result_count, filters_json],
        )?;
        Ok(())
    }

    /// Get recent searches (last 20)
    pub fn get_recent_searches(&self, limit: usize) -> SqlResult<Vec<SearchHistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT query, timestamp, result_count, filters 
             FROM search_history 
             ORDER BY timestamp DESC 
             LIMIT ?",
        )?;

        let entries = stmt
            .query_map(params![limit], |row| {
                let filters_json: String = row.get(3)?;
                let filters = serde_json::from_str(&filters_json).unwrap_or_default();
                Ok(SearchHistoryEntry {
                    query: row.get(0)?,
                    timestamp: row.get(1)?,
                    result_count: row.get(2)?,
                    filters,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(entries)
    }

    /// Get popular searches (top 10 by frequency)
    pub fn get_popular_searches(&self) -> SqlResult<Vec<(String, usize)>> {
        let mut stmt = self.conn.prepare(
            "SELECT query, COUNT(*) as cnt 
             FROM search_history 
             WHERE query != '' 
             GROUP BY LOWER(query) 
             ORDER BY cnt DESC 
             LIMIT 10",
        )?;

        let searches = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(searches)
    }

    /// Clear all search history
    pub fn clear_history(&mut self) -> SqlResult<()> {
        self.conn.execute("DELETE FROM search_history", [])?;
        Ok(())
    }
}

/// Generate autocomplete suggestions from patterns, sources, and recent searches
pub fn get_search_suggestions(
    query: &str,
    recent_searches: &[SearchHistoryEntry],
    available_sources: &[String],
) -> Vec<SearchSuggestion> {
    let query_lower = query.to_lowercase();
    let mut suggestions = Vec::new();

    // Pattern-based suggestions (common search terms)
    const PATTERN_KEYWORDS: &[&str] = &[
        "aws", "github", "stripe", "discord", "telegram", "jwt", "api key",
        "password", "token", "private key", "ssh", "webhook", "oauth",
        "database", "mongodb", "mysql", "postgres", "redis", "s3",
        "credit card", "email", "combo", "stealer", "dump", "breach",
    ];

    for keyword in PATTERN_KEYWORDS {
        if keyword.contains(&query_lower) || query_lower.contains(keyword) {
            suggestions.push(SearchSuggestion {
                text: keyword.to_string(),
                category: "pattern".to_string(),
                count: None,
            });
        }
    }

    // Source-based suggestions
    for source in available_sources {
        if source.to_lowercase().contains(&query_lower) {
            suggestions.push(SearchSuggestion {
                text: format!("source:{}", source),
                category: "source".to_string(),
                count: None,
            });
        }
    }

    // Recent search suggestions
    for entry in recent_searches.iter().take(5) {
        if entry.query.to_lowercase().contains(&query_lower) && !entry.query.is_empty() {
            suggestions.push(SearchSuggestion {
                text: entry.query.clone(),
                category: "recent".to_string(),
                count: Some(entry.result_count),
            });
        }
    }

    // Deduplicate and limit to 10
    suggestions.sort_by(|a, b| a.text.cmp(&b.text));
    suggestions.dedup_by(|a, b| a.text == b.text);
    suggestions.truncate(10);
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_history() {
        let mut history = SearchHistory::new(":memory:").unwrap();

        let entry = SearchHistoryEntry {
            query: "aws key".to_string(),
            timestamp: 1700000000,
            result_count: 42,
            filters: SearchFilters {
                is_sensitive: Some(true),
                ..Default::default()
            },
        };

        history.record_search(&entry).unwrap();
        let recent = history.get_recent_searches(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].query, "aws key");
        assert_eq!(recent[0].result_count, 42);
    }

    #[test]
    fn test_search_suggestions() {
        let recent = vec![
            SearchHistoryEntry {
                query: "github token".to_string(),
                timestamp: 1700000000,
                result_count: 10,
                filters: Default::default(),
            },
        ];
        let sources = vec!["pastebin".to_string(), "ghostbin".to_string()];

        let suggestions = get_search_suggestions("git", &recent, &sources);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.category == "recent"));
    }
}
