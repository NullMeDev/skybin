pub mod traits;
pub mod pastebin;
pub mod github_gists;
pub mod paste_ee;
pub mod dpaste;

pub use traits::{Scraper, ScraperError, ScraperResult};
pub use pastebin::PastebinScraper;
pub use github_gists::GitHubGistsScraper;
pub use paste_ee::PasteEeScraper;
pub use dpaste::DPasteScraper;
