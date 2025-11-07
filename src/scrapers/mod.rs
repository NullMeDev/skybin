pub mod traits;
pub mod pastebin;

pub use traits::{Scraper, ScraperError, ScraperResult};
pub use pastebin::PastebinScraper;
