use serde::{Deserialize, Serialize};

/// Represents a paste stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paste {
    pub id: String,
    pub source: String,
    pub source_id: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub content: String,
    pub content_hash: String,
    pub url: Option<String>,
    pub syntax: String,
    pub matched_patterns: Option<Vec<PatternMatch>>,
    pub is_sensitive: bool,
    pub created_at: i64,
    pub expires_at: i64,
    pub view_count: i64,
}

/// Represents a detected pattern match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub name: String,
    pub snippet: String,
    pub severity: String,
}

/// Represents a paste discovered during scraping (before storage)
#[derive(Debug, Clone)]
pub struct DiscoveredPaste {
    pub source: String,
    pub source_id: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub content: String,
    pub url: String,
    pub syntax: Option<String>,
    pub created_at: Option<i64>,
}

impl DiscoveredPaste {
    pub fn new(source: impl Into<String>, source_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            source_id: source_id.into(),
            content: content.into(),
            title: None,
            author: None,
            url: String::new(),
            syntax: None,
            created_at: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    pub fn with_syntax(mut self, syntax: impl Into<String>) -> Self {
        self.syntax = Some(syntax.into());
        self
    }

    pub fn with_created_at(mut self, timestamp: i64) -> Self {
        self.created_at = Some(timestamp);
        self
    }
}

/// Search filters for querying pastes
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SearchFilters {
    pub query: Option<String>,
    pub source: Option<String>,
    pub is_sensitive: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Statistics about the pastebin
#[derive(Debug, Serialize)]
pub struct Stats {
    pub total_pastes: i64,
    pub sensitive_pastes: i64,
    pub sources: Vec<SourceStat>,
}

#[derive(Debug, Serialize)]
pub struct SourceStat {
    pub source: String,
    pub count: i64,
}
