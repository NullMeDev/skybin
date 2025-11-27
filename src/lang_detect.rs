/// Simple language detection based on common patterns
pub fn detect_language(content: &str) -> String {
    let content_lower = content.to_lowercase();

    // HTML/XML
    if content.contains("<!DOCTYPE") || content.contains("<html") || content.contains("</html>") {
        return "HTML".to_string();
    }

    // CSS
    if content.contains("{")
        && content.contains("}")
        && (content.contains(":") && content.contains(";"))
        && (content_lower.contains("color:")
            || content_lower.contains("font-")
            || content_lower.contains("margin"))
    {
        return "CSS".to_string();
    }

    // JavaScript/TypeScript
    if content_lower.contains("function")
        || content_lower.contains("const ")
        || content_lower.contains("let ")
        || content_lower.contains("var ")
        || content_lower.contains("=>")
        || content_lower.contains("console.log")
    {
        if content_lower.contains("interface ")
            || content_lower.contains(": string")
            || content_lower.contains(": number")
        {
            return "TypeScript".to_string();
        }
        return "JavaScript".to_string();
    }

    // Python
    if content_lower.contains("def ")
        || content_lower.contains("import ")
        || content_lower.contains("from ")
        || content_lower.contains("print(")
        || content_lower.contains("if __name__")
    {
        return "Python".to_string();
    }

    // Java
    if content_lower.contains("public class ")
        || content_lower.contains("private ")
        || content_lower.contains("public static void main")
    {
        return "Java".to_string();
    }

    // C#
    if content_lower.contains("using System")
        || content_lower.contains("namespace ") && content_lower.contains("class ")
    {
        return "C#".to_string();
    }

    // C/C++
    if content.contains("#include") || content_lower.contains("int main(") {
        if content.contains("#include <iostream>") || content_lower.contains("std::") {
            return "C++".to_string();
        }
        return "C".to_string();
    }

    // Rust
    if content_lower.contains("fn main()")
        || content_lower.contains("let mut ")
        || content.contains("impl ")
    {
        return "Rust".to_string();
    }

    // Go
    if content_lower.contains("package main")
        || content_lower.contains("func main()")
        || content.contains("import (")
    {
        return "Go".to_string();
    }

    // PHP
    if content.starts_with("<?php") || content.contains("<?php") {
        return "PHP".to_string();
    }

    // Ruby
    if content_lower.contains("def ")
        && (content_lower.contains("end\n") || content_lower.contains("end "))
    {
        return "Ruby".to_string();
    }

    // SQL
    if content_lower.contains("select ") && content_lower.contains("from ")
        || content_lower.contains("insert into")
        || content_lower.contains("create table")
    {
        return "SQL".to_string();
    }

    // JSON
    if ((content.trim().starts_with("{") && content.trim().ends_with("}"))
        || (content.trim().starts_with("[") && content.trim().ends_with("]")))
        && (content.contains("\":") || content.contains("\": "))
    {
        return "JSON".to_string();
    }

    // Markdown
    if content.contains("# ") || content.contains("## ") || content.contains("```") {
        return "Markdown".to_string();
    }

    // YAML
    if content_lower.contains("---") && content.contains(":") && !content.contains(";") {
        return "YAML".to_string();
    }

    // Shell/Bash
    if content.starts_with("#!")
        || content_lower.contains("#!/bin/bash")
        || content_lower.contains("#!/bin/sh")
    {
        return "Shell".to_string();
    }

    "Plaintext".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_python() {
        let code = "def hello():\n    print('Hello')";
        assert_eq!(detect_language(code), "Python");
    }

    #[test]
    fn test_detect_javascript() {
        let code = "function hello() {\n    console.log('Hello');\n}";
        assert_eq!(detect_language(code), "JavaScript");
    }

    #[test]
    fn test_detect_json() {
        let code = r#"{"name": "test", "value": 123}"#;
        assert_eq!(detect_language(code), "JSON");
    }

    #[test]
    fn test_detect_plaintext() {
        let code = "This is just plain text with no code.";
        assert_eq!(detect_language(code), "Plaintext");
    }
}
