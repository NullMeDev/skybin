use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScraperError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Source unavailable: {0}")]
    SourceUnavailable(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type ScraperResult<T> = Result<T, ScraperError>;

/// Base trait for paste scrapers
#[async_trait]
pub trait Scraper: Send + Sync {
    /// Source name (e.g., "pastebin", "gists")
    fn name(&self) -> &str;

    /// Fetch recent pastes from the source
    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>>;
}
