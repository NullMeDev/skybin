use crate::models::DiscoveredPaste;
use async_trait::async_trait;
use super::traits::{Scraper, ScraperResult};

pub struct IdeoneScraper;

impl IdeoneScraper {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IdeoneScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for IdeoneScraper {
    fn name(&self) -> &str {
        "ideone"
    }

    async fn fetch_recent(&self, client: &reqwest::Client) -> ScraperResult<Vec<DiscoveredPaste>> {
        let resp = client
            .get("https://ideone.com/recent")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;
        
        let html = resp.text().await?;
        let mut pastes = Vec::new();
        let re = regex::Regex::new(r#"href="/([a-zA-Z0-9]{6})""#).unwrap();
        
        for cap in re.captures_iter(&html).take(15) {
            if let Some(id) = cap.get(1) {
                let paste_id = id.as_str();
                let raw_url = format!("https://ideone.com/plain/{}", paste_id);
                if let Ok(content_resp) = client.get(&raw_url)
                    .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                    .send()
                    .await
                {
                    if let Ok(content) = content_resp.text().await {
                        // Skip trivial code snippets (Hello World, basic exercises)
                        if !content.is_empty() && content.len() < 100000 && content.len() > 100 
                            && !is_trivial_code(&content) {
                            pastes.push(DiscoveredPaste::new("ideone", paste_id, content)
                                .with_url(format!("https://ideone.com/{}", paste_id)));
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
            }
        }
        
        Ok(pastes)
    }
}

/// Filter out trivial student code snippets
fn is_trivial_code(content: &str) -> bool {
    let content_lower = content.to_lowercase();
    let line_count = content.lines().count();
    
    // Too short to be interesting
    if line_count < 5 {
        return true;
    }
    
    // Classic Hello World indicators
    let trivial_patterns = [
        "hello world",
        "hello, world",
        "helloworld",
        "print(\"hello",
        "printf(\"hello",
        "cout << \"hello",
        "system.out.println(\"hello",
        "console.log(\"hello",
        "echo \"hello",
        "puts \"hello",
        // Common student exercises
        "fibonacci",
        "factorial",
        "prime number",
        "bubble sort",
        "binary search",
        "linked list",
        "calculator",
        "sum of",
        "average of",
        "armstrong",
        "palindrome",
        "reverse string",
        "swap two",
        // Test/placeholder code
        "lorem ipsum",
        "test123",
        "asdf",
        "qwerty",
        "// todo",
        "# todo",
    ];
    
    for pattern in trivial_patterns {
        if content_lower.contains(pattern) {
            return true;
        }
    }
    
    // If it's just a basic #include with main() and nothing interesting
    if content_lower.contains("#include") && 
       content_lower.contains("int main") &&
       line_count < 20 &&
       !content_lower.contains("@") &&  // No emails
       !content_lower.contains("password") &&
       !content_lower.contains("api") &&
       !content_lower.contains("token") &&
       !content_lower.contains("secret") &&
       !content_lower.contains("key") {
        return true;
    }
    
    false
}
