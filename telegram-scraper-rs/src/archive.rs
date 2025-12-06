use std::io::{Read, Cursor};
use std::path::Path;
use tracing::{info, debug, warn};
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    /// Password file patterns (case-insensitive matching)
    static ref PASSWORD_FILE_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)^passwords?\.txt$").unwrap(),
        Regex::new(r"(?i)^pass(wd)?\.txt$").unwrap(),
        Regex::new(r"(?i)^pwd\.txt$").unwrap(),
        Regex::new(r"(?i)^logins?\.txt$").unwrap(),
        Regex::new(r"(?i)^credentials?\.txt$").unwrap(),
        Regex::new(r"(?i)^combo\.txt$").unwrap(),
        Regex::new(r"(?i)^accounts?\.txt$").unwrap(),
        Regex::new(r"(?i)^all\s*passwords?\.txt$").unwrap(),
        Regex::new(r"(?i)passwords?.*\.txt$").unwrap(),
    ];
    
    /// Credential patterns for validation
    static ref EMAIL_PASS_PATTERN: Regex = Regex::new(
        r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@:]{4,}"
    ).unwrap();
    
    static ref URL_LOGIN_PASS_PATTERN: Regex = Regex::new(
        r"https?://[^\s]+[\s\t|:]+[^\s@]+[\s\t|:]+[^\s]{4,}"
    ).unwrap();
}

/// Extracted password file content
#[derive(Debug, Clone)]
pub struct ExtractedPassword {
    pub content: String,
    pub filename: String,
    pub email_pass_count: usize,
    pub url_login_count: usize,
    pub line_count: usize,
}

/// Check if a filename is a password file
pub fn is_password_file(filename: &str) -> bool {
    let basename = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filename);
    
    PASSWORD_FILE_PATTERNS.iter().any(|p| p.is_match(basename))
}

/// Check if content contains valid credentials
pub fn has_valid_credentials(content: &str) -> bool {
    let email_count = EMAIL_PASS_PATTERN.find_iter(content).count();
    let url_count = URL_LOGIN_PASS_PATTERN.find_iter(content).count();
    
    // Accept any credential presence (≥1)
    email_count >= 1 || url_count >= 1
}

/// Count credentials in content
pub fn count_credentials(content: &str) -> (usize, usize) {
    let email_count = EMAIL_PASS_PATTERN.find_iter(content).count();
    let url_count = URL_LOGIN_PASS_PATTERN.find_iter(content).count();
    (email_count, url_count)
}

/// Check if file is an archive
pub fn is_archive(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".zip") 
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tbz2")
}

/// Extract password files from archive data
/// Returns all found password files with their content
/// Supports nested archives (up to 2 levels deep)
pub fn extract_password_files(
    data: &[u8],
    filename: &str,
    max_file_size: u64,
    depth: usize,
) -> Vec<ExtractedPassword> {
    let max_depth = 2;
    if depth > max_depth {
        debug!("Max archive nesting depth reached");
        return vec![];
    }
    
    let lower = filename.to_lowercase();
    
    if lower.ends_with(".zip") {
        extract_from_zip(data, max_file_size, depth)
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        extract_from_tar_gz(data, max_file_size, depth)
    } else if lower.ends_with(".tar.bz2") || lower.ends_with(".tbz2") {
        extract_from_tar_bz2(data, max_file_size, depth)
    } else {
        vec![]
    }
}

fn extract_from_zip(data: &[u8], max_file_size: u64, depth: usize) -> Vec<ExtractedPassword> {
    let mut results = vec![];
    let cursor = Cursor::new(data);
    
    let Ok(mut archive) = zip::ZipArchive::new(cursor) else {
        warn!("Failed to open zip archive");
        return results;
    };
    
    for i in 0..archive.len() {
        let Ok(mut file) = archive.by_index(i) else { continue };
        
        if file.is_dir() {
            continue;
        }
        
        let name = file.name().to_string();
        let size = file.size();
        
        if size > max_file_size * 1024 * 1024 {
            debug!("Skipping large file: {} ({} MB)", name, size / 1024 / 1024);
            continue;
        }
        
        // Check for nested archives
        if is_archive(&name) && depth < 2 {
            let mut nested_data = Vec::new();
            if file.read_to_end(&mut nested_data).is_ok() {
                info!("  📦 Found nested archive: {}", name);
                let nested = extract_password_files(&nested_data, &name, max_file_size, depth + 1);
                results.extend(nested);
            }
            continue;
        }
        
        // Check if it's a password file
        if !is_password_file(&name) {
            continue;
        }
        
        info!("  🔑 Found password file: {}", name);
        
        let mut content = String::new();
        if file.read_to_string(&mut content).is_err() {
            // Try reading as bytes and converting
            let mut bytes = Vec::new();
            if file.read_to_end(&mut bytes).is_ok() {
                content = String::from_utf8_lossy(&bytes).to_string();
            }
        }
        
        if content.trim().len() < 50 {
            debug!("Skipping empty/small password file: {}", name);
            continue;
        }
        
        let (email_count, url_count) = count_credentials(&content);
        let line_count = content.lines().count();
        
        results.push(ExtractedPassword {
            content,
            filename: name,
            email_pass_count: email_count,
            url_login_count: url_count,
            line_count,
        });
    }
    
    results
}

fn extract_from_tar_gz(data: &[u8], max_file_size: u64, depth: usize) -> Vec<ExtractedPassword> {
    let mut results = vec![];
    
    let cursor = Cursor::new(data);
    let decoder = flate2::read::GzDecoder::new(cursor);
    
    let mut archive = tar::Archive::new(decoder);
    let Ok(entries) = archive.entries() else {
        warn!("Failed to read tar entries");
        return results;
    };
    
    for entry in entries.flatten() {
        let Ok(path) = entry.path() else { continue };
        let name = path.to_string_lossy().to_string();
        let size = entry.size();
        
        if size > max_file_size * 1024 * 1024 {
            continue;
        }
        
        let mut content_bytes = Vec::new();
        let mut entry = entry;
        if entry.read_to_end(&mut content_bytes).is_err() {
            continue;
        }
        
        if is_archive(&name) && depth < 2 {
            info!("  📦 Found nested archive: {}", name);
            let nested = extract_password_files(&content_bytes, &name, max_file_size, depth + 1);
            results.extend(nested);
        } else if is_password_file(&name) {
            info!("  🔑 Found password file: {}", name);
            let content = String::from_utf8_lossy(&content_bytes).to_string();
            
            if content.trim().len() >= 50 {
                let (email_count, url_count) = count_credentials(&content);
                let line_count = content.lines().count();
                
                results.push(ExtractedPassword {
                    content,
                    filename: name,
                    email_pass_count: email_count,
                    url_login_count: url_count,
                    line_count,
                });
            }
        }
    }
    
    results
}

fn extract_from_tar_bz2(data: &[u8], max_file_size: u64, depth: usize) -> Vec<ExtractedPassword> {
    let mut results = vec![];
    
    let cursor = Cursor::new(data);
    let decoder = bzip2::read::BzDecoder::new(cursor);
    
    let mut archive = tar::Archive::new(decoder);
    let Ok(entries) = archive.entries() else {
        warn!("Failed to read tar.bz2 entries");
        return results;
    };
    
    for entry in entries.flatten() {
        let Ok(path) = entry.path() else { continue };
        let name = path.to_string_lossy().to_string();
        let size = entry.size();
        
        if size > max_file_size * 1024 * 1024 {
            continue;
        }
        
        let mut content_bytes = Vec::new();
        let mut entry = entry;
        if entry.read_to_end(&mut content_bytes).is_err() {
            continue;
        }
        
        if is_archive(&name) && depth < 2 {
            info!("  📦 Found nested archive: {}", name);
            let nested = extract_password_files(&content_bytes, &name, max_file_size, depth + 1);
            results.extend(nested);
        } else if is_password_file(&name) {
            info!("  🔑 Found password file: {}", name);
            let content = String::from_utf8_lossy(&content_bytes).to_string();
            
            if content.trim().len() >= 50 {
                let (email_count, url_count) = count_credentials(&content);
                let line_count = content.lines().count();
                
                results.push(ExtractedPassword {
                    content,
                    filename: name,
                    email_pass_count: email_count,
                    url_login_count: url_count,
                    line_count,
                });
            }
        }
    }
    
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_password_file() {
        assert!(is_password_file("passwords.txt"));
        assert!(is_password_file("Passwords.txt"));
        assert!(is_password_file("PASSWORDS.TXT"));
        assert!(is_password_file("password.txt"));
        assert!(is_password_file("logins.txt"));
        assert!(is_password_file("credentials.txt"));
        assert!(is_password_file("combo.txt"));
        assert!(is_password_file("accounts.txt"));
        assert!(is_password_file("All Passwords.txt"));
        assert!(is_password_file("Passwords_2024.txt"));
        
        assert!(!is_password_file("readme.txt"));
        assert!(!is_password_file("data.csv"));
        assert!(!is_password_file("image.png"));
    }
    
    #[test]
    fn test_is_archive() {
        assert!(is_archive("file.zip"));
        assert!(is_archive("file.tar.gz"));
        assert!(is_archive("file.tgz"));
        assert!(is_archive("file.tar.bz2"));
        
        assert!(!is_archive("file.txt"));
        assert!(!is_archive("file.exe"));
        assert!(!is_archive("file.rar")); // Not supported yet
        assert!(!is_archive("file.7z")); // Not supported yet
    }
}
