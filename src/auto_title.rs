use regex::Regex;

pub fn generate_title(content: &str) -> String {
    let content = content.trim();
    
    if content.is_empty() {
        return "Empty Paste".to_string();
    }

    if let Some(title) = detect_code_type(content) {
        return title;
    }

    if let Some(title) = extract_first_meaningful_line(content) {
        return title;
    }

    if let Some(title) = detect_data_type(content) {
        return title;
    }

    generate_summary(content)
}

fn detect_code_type(content: &str) -> Option<String> {
    let patterns: &[(&str, &str)] = &[
        (r#"^\s*<\?php"#, "PHP Script"),
        (r#"^\s*#!/usr/bin/(env\s+)?python"#, "Python Script"),
        (r#"^\s*#!/usr/bin/(env\s+)?bash"#, "Bash Script"),
        (r#"^\s*#!/usr/bin/(env\s+)?node"#, "Node.js Script"),
        (r#"^\s*#!/usr/bin/(env\s+)?ruby"#, "Ruby Script"),
        (r#"^\s*#!/usr/bin/(env\s+)?perl"#, "Perl Script"),
        (r#"^\s*package\s+main"#, "Go Program"),
        (r#"^\s*fn\s+main\s*\("#, "Rust Program"),
        (r#"^\s*public\s+class\s+\w+"#, "Java Class"),
        (r#"^\s*class\s+\w+.*:"#, "Python Class"),
        (r#"^\s*import\s+(React|useState|useEffect)"#, "React Component"),
        (r#"^\s*import\s+\{.*\}\s+from\s+['"]vue['"]"#, "Vue Component"),
        (r#"^\s*<template>"#, "Vue Template"),
        (r#"^\s*<!DOCTYPE\s+html>"#, "HTML Document"),
        (r#"^\s*<html"#, "HTML Document"),
        (r#"^\s*\{[\s\n]*""#, "JSON Data"),
        (r#"^\s*---\n"#, "YAML Document"),
        (r#"^\s*#\s+\w+"#, "Markdown Document"),
        (r#"^\s*CREATE\s+TABLE"#, "SQL Schema"),
        (r#"^\s*SELECT\s+"#, "SQL Query"),
        (r#"^\s*INSERT\s+INTO"#, "SQL Insert"),
        (r#"^\s*UPDATE\s+\w+\s+SET"#, "SQL Update"),
        (r#"^\s*const\s+\w+\s*=\s*require\("#, "Node.js Module"),
        (r#"^\s*import\s+\w+\s+from\s+"#, "ES6 Module"),
        (r#"^\s*export\s+(default\s+)?(function|class|const)"#, "ES6 Export"),
        (r#"^\s*\[Unit\]"#, "Systemd Unit File"),
        (r#"^\s*\[Service\]"#, "Systemd Service"),
        (r#"^\s*FROM\s+\w+"#, "Dockerfile"),
        (r#"^\s*version:\s*['"]?\d"#, "Docker Compose"),
        (r#"^\s*apiVersion:"#, "Kubernetes Manifest"),
        (r#"^\s*terraform\s*\{"#, "Terraform Config"),
        (r#"^\s*provider\s+""#, "Terraform Provider"),
        (r#"^\s*resource\s+""#, "Terraform Resource"),
    ];

    for (pattern, name) in patterns {
        if let Ok(re) = Regex::new(&format!("(?i){}", pattern)) {
            if re.is_match(content) {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn detect_data_type(content: &str) -> Option<String> {
    let patterns = [
        (r"(?i)api[_-]?key\s*[:=]", "API Key Leak"),
        (r"(?i)password\s*[:=]", "Password Data"),
        (r"(?i)secret[_-]?key\s*[:=]", "Secret Key Data"),
        (r"AKIA[0-9A-Z]{16}", "AWS Credentials"),
        (r"-----BEGIN\s+(RSA|DSA|EC|OPENSSH)\s+PRIVATE\s+KEY-----", "Private Key"),
        (r"-----BEGIN\s+CERTIFICATE-----", "SSL Certificate"),
        (r"ghp_[a-zA-Z0-9]{36}", "GitHub Token"),
        (r"xox[baprs]-[0-9a-zA-Z-]+", "Slack Token"),
        (r"(?i)mysql://|postgres://|mongodb://", "Database Connection String"),
        (r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b.*\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b", "IP Address List"),
        (r"([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}.*){3,}", "Email List"),
        (r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\s*:\s*\S+", "Email:Password Combo List"),
        (r"\b4[0-9]{12}(?:[0-9]{3})?\b", "Credit Card Numbers"),
        (r"(?i)bearer\s+[a-zA-Z0-9._-]+", "Bearer Token"),
        (r"(?i)authorization:\s*", "Authorization Header"),
        (r"(?i)(error|exception|traceback|stack\s*trace)", "Error Log"),
        (r"(?i)access[_-]?log|error[_-]?log", "Server Log"),
        (r"\[\d{2}/\w{3}/\d{4}:\d{2}:\d{2}:\d{2}", "Apache/Nginx Log"),
    ];

    for (pattern, name) in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(content) {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn extract_first_meaningful_line(content: &str) -> Option<String> {
    for line in content.lines().take(10) {
        let line = line.trim();
        
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') && !line.starts_with("##") {
            let title = line.trim_start_matches('#').trim();
            if !title.is_empty() && title.len() <= 60 {
                return Some(title.to_string());
            }
        }
        if line.starts_with("//") || line.starts_with("/*") || line.starts_with("*") {
            let cleaned = line
                .trim_start_matches('/')
                .trim_start_matches('*')
                .trim();
            if cleaned.len() >= 10 && cleaned.len() <= 60 && !cleaned.contains("TODO") {
                return Some(cleaned.to_string());
            }
        }
        if let Some(caps) = Regex::new(r#"^(?:def|function|fn|class|struct|interface|type)\s+(\w+)"#)
            .ok()
            .and_then(|re| re.captures(line))
        {
            if let Some(name) = caps.get(1) {
                return Some(format!("{} Definition", name.as_str()));
            }
        }
    }
    None
}

fn generate_summary(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or("").trim();
    
    if first_line.is_empty() {
        return "Text Paste".to_string();
    }

    let cleaned: String = first_line
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
        .collect();
    
    let cleaned = cleaned.trim();
    
    if cleaned.len() < 3 {
        return "Code Snippet".to_string();
    }

    if cleaned.len() > 50 {
        let truncated: String = cleaned.chars().take(47).collect();
        format!("{}...", truncated.trim_end())
    } else {
        cleaned.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_php_detection() {
        assert_eq!(generate_title("<?php echo 'hello';"), "PHP Script");
    }

    #[test]
    fn test_python_detection() {
        assert_eq!(generate_title("#!/usr/bin/env python\nprint('hi')"), "Python Script");
    }

    #[test]
    fn test_json_detection() {
        assert_eq!(generate_title("{\"key\": \"value\"}"), "JSON Data");
    }

    #[test]
    fn test_aws_key_detection() {
        assert_eq!(generate_title("AKIAIOSFODNN7EXAMPLE"), "AWS Credentials");
    }

    #[test]
    fn test_markdown_title() {
        assert_eq!(generate_title("# My Document\n\nContent here"), "My Document");
    }

    #[test]
    fn test_empty() {
        assert_eq!(generate_title(""), "Empty Paste");
    }
}
