pub mod dpaste;
pub mod github_gists;
pub mod paste_ee;
pub mod pastebin;
pub mod traits;

pub use dpaste::DPasteScraper;
pub use github_gists::GitHubGistsScraper;
pub use paste_ee::PasteEeScraper;
pub use pastebin::PastebinScraper;
pub use traits::{Scraper, ScraperError, ScraperResult};
