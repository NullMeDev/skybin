use regex::Regex;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref EMAIL_PASS_PATTERN: Regex = Regex::new(
        r"([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+):([^\s@:]{4,})"
    ).unwrap();
    
    static ref URL_LOGIN_PASS_PATTERN: Regex = Regex::new(
        r"(https?://[^\s]+)[\s\t|:]+([^\s@]+)[\s\t|:]+([^\s]{4,})"
    ).unwrap();
    
    static ref DOMAIN_EXTRACT: Regex = Regex::new(
        r"https?://(?:www\.)?([^/:\s]+)"
    ).unwrap();
}

/// Service classification result
#[derive(Debug, Clone)]
pub struct ServiceStats {
    pub total_credentials: usize,
    pub email_pass_count: usize,
    pub url_login_count: usize,
    pub service_counts: HashMap<String, usize>,
}

/// Classify credentials in content by service
pub fn classify_credentials(content: &str) -> ServiceStats {
    let mut service_counts: HashMap<String, usize> = HashMap::new();
    let mut total_credentials = 0;
    let mut email_pass_count = 0;
    let mut url_login_count = 0;
    
    // Classify email:pass credentials by domain
    for caps in EMAIL_PASS_PATTERN.captures_iter(content) {
        if let Some(email_match) = caps.get(1) {
            let email = email_match.as_str();
            if let Some(domain) = extract_email_domain(email) {
                let service = classify_email_domain(&domain);
                *service_counts.entry(service).or_insert(0) += 1;
                total_credentials += 1;
                email_pass_count += 1;
            }
        }
    }
    
    // Classify URL:login:pass credentials by domain
    for caps in URL_LOGIN_PASS_PATTERN.captures_iter(content) {
        if let Some(url_match) = caps.get(1) {
            let url = url_match.as_str();
            if let Some(domain) = extract_url_domain(url) {
                let service = classify_url_domain(&domain);
                *service_counts.entry(service).or_insert(0) += 1;
                total_credentials += 1;
                url_login_count += 1;
            }
        }
    }
    
    ServiceStats {
        total_credentials,
        email_pass_count,
        url_login_count,
        service_counts,
    }
}

/// Extract domain from email address
fn extract_email_domain(email: &str) -> Option<String> {
    email.split('@').nth(1).map(|d| d.to_lowercase())
}

/// Extract domain from URL
fn extract_url_domain(url: &str) -> Option<String> {
    DOMAIN_EXTRACT.captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_lowercase())
}

/// Classify email domain to service name
fn classify_email_domain(domain: &str) -> String {
    let domain = domain.to_lowercase();
    
    // Email provider mapping
    match domain.as_str() {
        // Gmail
        d if d == "gmail.com" || d.ends_with(".gmail.com") => "Gmail".to_string(),
        
        // Microsoft
        d if d == "outlook.com" || d == "hotmail.com" || d == "live.com" 
            || d == "msn.com" || d.ends_with(".outlook.com") => "Outlook".to_string(),
        
        // Yahoo
        d if d == "yahoo.com" || d.ends_with(".yahoo.com") 
            || d == "ymail.com" || d == "rocketmail.com" => "Yahoo".to_string(),
        
        // ProtonMail
        d if d == "protonmail.com" || d == "proton.me" 
            || d == "pm.me" => "ProtonMail".to_string(),
        
        // iCloud
        d if d == "icloud.com" || d == "me.com" || d == "mac.com" => "iCloud".to_string(),
        
        // AOL
        d if d == "aol.com" || d.ends_with(".aol.com") => "AOL".to_string(),
        
        // Zoho
        d if d == "zoho.com" || d.ends_with(".zoho.com") => "Zoho".to_string(),
        
        // Yandex
        d if d == "yandex.com" || d == "yandex.ru" || d.ends_with(".yandex.") => "Yandex".to_string(),
        
        // Mail.ru
        d if d == "mail.ru" || d.ends_with(".mail.ru") => "Mail.ru".to_string(),
        
        // GMX
        d if d == "gmx.com" || d == "gmx.net" || d.ends_with(".gmx.") => "GMX".to_string(),
        
        // Default: use domain as-is
        _ => format!("Email ({})", domain.split('.').next().unwrap_or(&domain))
    }
}

/// Classify URL domain to service name
fn classify_url_domain(domain: &str) -> String {
    let domain = domain.to_lowercase();
    
    match domain.as_str() {
        // Gaming
        d if d.contains("roblox") => "Roblox".to_string(),
        d if d.contains("steam") => "Steam".to_string(),
        d if d.contains("epic") => "Epic Games".to_string(),
        d if d.contains("minecraft") => "Minecraft".to_string(),
        d if d.contains("fortnite") => "Fortnite".to_string(),
        d if d.contains("playstation") || d.contains("psn") => "PlayStation".to_string(),
        d if d.contains("xbox") => "Xbox".to_string(),
        d if d.contains("battlenet") || d.contains("battle.net") => "Battle.net".to_string(),
        d if d.contains("origin") && d.contains("ea") => "EA Origin".to_string(),
        
        // Streaming
        d if d.contains("netflix") => "Netflix".to_string(),
        d if d.contains("spotify") => "Spotify".to_string(),
        d if d.contains("hulu") => "Hulu".to_string(),
        d if d.contains("disney") => "Disney+".to_string(),
        d if d.contains("hbo") => "HBO Max".to_string(),
        d if d.contains("paramount") => "Paramount+".to_string(),
        d if d.contains("crunchyroll") => "Crunchyroll".to_string(),
        d if d.contains("prime") || (d.contains("amazon") && d.contains("video")) => "Prime Video".to_string(),
        d if d.contains("apple") && d.contains("tv") => "Apple TV+".to_string(),
        
        // Social Media
        d if d.contains("facebook") || d == "fb.com" => "Facebook".to_string(),
        d if d.contains("instagram") => "Instagram".to_string(),
        d if d.contains("twitter") || d == "x.com" => "Twitter".to_string(),
        d if d.contains("tiktok") => "TikTok".to_string(),
        d if d.contains("snapchat") => "Snapchat".to_string(),
        d if d.contains("linkedin") => "LinkedIn".to_string(),
        d if d.contains("reddit") => "Reddit".to_string(),
        d if d.contains("discord") => "Discord".to_string(),
        d if d.contains("telegram") => "Telegram".to_string(),
        
        // E-commerce
        d if d.contains("amazon") => "Amazon".to_string(),
        d if d.contains("ebay") => "eBay".to_string(),
        d if d.contains("paypal") => "PayPal".to_string(),
        d if d.contains("shopify") => "Shopify".to_string(),
        d if d.contains("etsy") => "Etsy".to_string(),
        
        // Finance
        d if d.contains("coinbase") => "Coinbase".to_string(),
        d if d.contains("binance") => "Binance".to_string(),
        d if d.contains("kraken") => "Kraken".to_string(),
        d if d.contains("robinhood") => "Robinhood".to_string(),
        
        // Development
        d if d.contains("github") => "GitHub".to_string(),
        d if d.contains("gitlab") => "GitLab".to_string(),
        d if d.contains("bitbucket") => "Bitbucket".to_string(),
        
        // Other
        d if d.contains("dropbox") => "Dropbox".to_string(),
        d if d.contains("google") => "Google".to_string(),
        d if d.contains("microsoft") => "Microsoft".to_string(),
        d if d.contains("apple") => "Apple".to_string(),
        
        // Default: use domain name
        _ => {
            let parts: Vec<&str> = domain.split('.').collect();
            if parts.len() >= 2 {
                // Capitalize first letter
                let name = parts[parts.len() - 2];
                let mut chars = name.chars();
                match chars.next() {
                    None => name.to_string(),
                    Some(f) => f.to_uppercase().chain(chars).collect(),
                }
            } else {
                domain.to_string()
            }
        }
    }
}

/// Generate title from service stats
pub fn generate_title(stats: &ServiceStats) -> String {
    if stats.total_credentials == 0 {
        return "Credentials".to_string();
    }
    
    // Sort services by count (descending)
    let mut services: Vec<(&String, &usize)> = stats.service_counts.iter().collect();
    services.sort_by(|a, b| b.1.cmp(a.1));
    
    // Check if one service dominates (≥70%)
    if let Some((top_service, top_count)) = services.first() {
        let percentage = (**top_count as f64) / (stats.total_credentials as f64);
        
        if percentage >= 0.7 {
            // Single dominant service
            return format!("{}x {} Logins", top_count, top_service);
        }
        
        // Check if top 2 services together are ≥70%
        if services.len() >= 2 {
            let second_count = *services[1].1;
            let top_two_total = **top_count + second_count;
            let top_two_percentage = (top_two_total as f64) / (stats.total_credentials as f64);
            
            if top_two_percentage >= 0.7 && second_count >= 3 {
                // Two dominant services
                return format!("{}x {}, {}x {} Logins", 
                    top_count, top_service,
                    second_count, services[1].0);
            }
        }
    }
    
    // Assorted
    format!("{}x Assorted Logins", stats.total_credentials)
}

/// Generate summary header for paste content
pub fn generate_summary_header(stats: &ServiceStats) -> String {
    let mut header = String::new();
    
    header.push_str(&format!("═══════════════════════════════════════════\n"));
    header.push_str(&format!("CREDENTIALS SUMMARY\n"));
    header.push_str(&format!("═══════════════════════════════════════════\n"));
    header.push_str(&format!("Total: {} credentials\n", stats.total_credentials));
    
    if stats.email_pass_count > 0 || stats.url_login_count > 0 {
        header.push_str(&format!("  • Email:Pass: {}\n", stats.email_pass_count));
        header.push_str(&format!("  • URL:Login:Pass: {}\n", stats.url_login_count));
    }
    
    if !stats.service_counts.is_empty() {
        header.push_str("\nBy Service (Top 10):\n");
        
        let mut services: Vec<(&String, &usize)> = stats.service_counts.iter().collect();
        services.sort_by(|a, b| b.1.cmp(a.1));
        
        for (i, (service, count)) in services.iter().take(10).enumerate() {
            header.push_str(&format!("  {}. {} - {} logins\n", i + 1, service, count));
        }
    }
    
    header.push_str(&format!("═══════════════════════════════════════════\n\n"));
    
    header
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_email_domain_extraction() {
        assert_eq!(extract_email_domain("user@gmail.com"), Some("gmail.com".to_string()));
        assert_eq!(extract_email_domain("test@outlook.com"), Some("outlook.com".to_string()));
    }
    
    #[test]
    fn test_url_domain_extraction() {
        assert_eq!(extract_url_domain("https://www.roblox.com/login"), Some("roblox.com".to_string()));
        assert_eq!(extract_url_domain("http://netflix.com"), Some("netflix.com".to_string()));
    }
    
    #[test]
    fn test_email_classification() {
        assert_eq!(classify_email_domain("gmail.com"), "Gmail");
        assert_eq!(classify_email_domain("outlook.com"), "Outlook");
        assert_eq!(classify_email_domain("yahoo.com"), "Yahoo");
    }
    
    #[test]
    fn test_url_classification() {
        assert_eq!(classify_url_domain("roblox.com"), "Roblox");
        assert_eq!(classify_url_domain("steamcommunity.com"), "Steam");
        assert_eq!(classify_url_domain("netflix.com"), "Netflix");
    }
    
    #[test]
    fn test_title_generation() {
        let mut stats = ServiceStats {
            total_credentials: 10,
            email_pass_count: 10,
            url_login_count: 0,
            service_counts: HashMap::new(),
        };
        
        // Single dominant service (80%)
        stats.service_counts.insert("Gmail".to_string(), 8);
        stats.service_counts.insert("Yahoo".to_string(), 2);
        assert_eq!(generate_title(&stats), "8x Gmail Logins");
        
        // Two services (50% + 30% = 80%)
        stats.service_counts.clear();
        stats.service_counts.insert("Gmail".to_string(), 5);
        stats.service_counts.insert("Outlook".to_string(), 3);
        stats.service_counts.insert("Yahoo".to_string(), 2);
        assert_eq!(generate_title(&stats), "5x Gmail, 3x Outlook Logins");
        
        // Assorted
        stats.service_counts.clear();
        stats.service_counts.insert("Gmail".to_string(), 3);
        stats.service_counts.insert("Outlook".to_string(), 3);
        stats.service_counts.insert("Yahoo".to_string(), 2);
        stats.service_counts.insert("ProtonMail".to_string(), 2);
        assert_eq!(generate_title(&stats), "10x Assorted Logins");
    }
}
