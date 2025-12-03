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

/// Filter out trivial student code snippets - very aggressive
fn is_trivial_code(content: &str) -> bool {
    let content_lower = content.to_lowercase();
    let line_count = content.lines().count();
    let content_len = content.len();
    
    // Too short to be interesting (require substantial content)
    if line_count < 15 || content_len < 500 {
        return true;
    }
    
    // If it starts with #include and is short, skip it - this is student code
    let first_line = content.lines().next().unwrap_or("").trim().to_lowercase();
    if first_line.starts_with("#include") && line_count < 50 {
        // Allow only if it contains sensitive patterns
        if !has_sensitive_content(&content_lower) {
            return true;
        }
    }
    
    // Competitive programming headers = immediate skip
    if content_lower.contains("bits/stdc++.h") {
        return true;
    }
    
    // Common competitive programming patterns
    let competitive_patterns = [
        "#define ll long",
        "#define int long",
        "using namespace std",
        "void solve(",
        "int t; cin >> t",
        "while(t--)",
        "ios_base::sync",
        "cin.tie",
        "cout.tie",
        "freopen(",
        "vector<int>",
        "vector<ll>",
        "pair<int",
        "#define pb push_back",
        "#define mp make_pair",
        "#define all(",
        "#define rep(",
        "#define for(",
        "mod = 1e9",
        "1000000007",
        "998244353",
    ];
    
    let mut competitive_matches = 0;
    for pattern in &competitive_patterns {
        if content_lower.contains(pattern) {
            competitive_matches += 1;
        }
    }
    // If more than 2 competitive programming patterns, skip
    if competitive_matches >= 2 {
        return true;
    }
    
    // Classic trivial patterns
    let trivial_patterns = [
        "hello world", "hello, world", "helloworld",
        "print(\"hello", "printf(\"hello", "cout << \"hello",
        "system.out.println(\"hello", "console.log(\"hello",
        "fibonacci", "factorial", "prime number", "bubble sort",
        "binary search", "linked list", "calculator",
        "armstrong", "palindrome", "reverse string", "swap two",
        "lorem ipsum", "test123", "asdf", "qwerty",
        "// practice", "// exercise", "// homework", "// assignment",
        "/* practice", "/* exercise", "/* homework",
        "gcd(", "lcm(", "power(", "modpow(",
        "sieve", "dfs(", "bfs(",
    ];
    
    for pattern in trivial_patterns {
        if content_lower.contains(pattern) {
            return true;
        }
    }
    
    // If it's pure code without any interesting content
    if content_lower.contains("#include") && content_lower.contains("int main") {
        if !has_sensitive_content(&content_lower) {
            return true;
        }
    }
    
    // Python/Java student code
    if (content_lower.contains("def main") || content_lower.contains("public static void main"))
       && line_count < 40 && !has_sensitive_content(&content_lower) {
        return true;
    }
    
    false
}

/// Check if content has sensitive/interesting patterns
fn has_sensitive_content(content: &str) -> bool {
    let sensitive_patterns = [
        "password", "passwd", "pwd",
        "api_key", "apikey", "api-key",
        "token", "bearer", "auth",
        "secret", "credential",
        "private_key", "privatekey",
        "aws_", "azure", "gcp",
        "mysql://", "postgres://", "mongodb://", "redis://",
        "smtp", "ftp://", "ssh://",
        "@gmail", "@yahoo", "@outlook", "@hotmail",
        "credit", "card", "cvv",
        "bitcoin", "ethereum", "wallet",
        "discord", "telegram", "slack",
        "netflix", "spotify", "disney", "hulu",
        "admin", "root", "sudo",
        "hack", "exploit", "vuln",
        "leak", "dump", "breach",
        ".env", "config", "settings",
    ];
    
    for pattern in sensitive_patterns {
        if content.contains(pattern) {
            return true;
        }
    }
    false
}
