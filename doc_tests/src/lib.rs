// Documentation testing utilities library
// This library provides helper functions for validating SECURITY.md structure and content

use regex::Regex;

/// Helper function to extract a section from markdown content
/// Sections are identified by markdown headers (## or ###)
pub fn extract_section(content: &str, section_name: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut section_content = String::new();
    let mut in_section = false;
    let mut section_level = 0;

    for line in lines {
        // Check if this is a header line
        if line.starts_with('#') {
            let current_level = line.chars().take_while(|&c| c == '#').count();
            let header_text = line.trim_start_matches('#').trim();

            // Check if this is the section we're looking for
            if header_text == section_name {
                in_section = true;
                section_level = current_level;
                section_content.push_str(line);
                section_content.push('\n');
                continue;
            }

            // If we're in a section and encounter a header of equal or higher level, we're done
            if in_section && current_level <= section_level {
                break;
            }
        }

        // Add line to section content if we're in the target section
        if in_section {
            section_content.push_str(line);
            section_content.push('\n');
        }
    }

    section_content
}

/// Helper function to check if content contains all required keywords
pub fn contains_all_keywords(content: &str, keywords: &[&str]) -> bool {
    let lowercase_content = content.to_lowercase();
    keywords.iter().all(|keyword| lowercase_content.contains(&keyword.to_lowercase()))
}

/// Helper function to extract URLs from markdown content
pub fn extract_urls(content: &str) -> Vec<String> {
    let url_pattern = Regex::new(r"https?://[^\s\)]+").unwrap();
    url_pattern
        .find_iter(content)
        .map(|m| m.as_str().to_string())
        .collect()
}

/// Helper function to check if a section exists in the content
pub fn section_exists(content: &str, section_name: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    for line in lines {
        if line.starts_with('#') {
            let header_text = line.trim_start_matches('#').trim();
            if header_text == section_name {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_section() {
        let content = "# Title\n## Section 1\nContent 1\n## Section 2\nContent 2";
        let section = extract_section(content, "Section 1");
        assert!(section.contains("Section 1"));
        assert!(section.contains("Content 1"));
        assert!(!section.contains("Section 2"));
    }

    #[test]
    fn test_contains_all_keywords() {
        let content = "This is a test with keywords: front-running and revocation";
        assert!(contains_all_keywords(content, &["front-running", "revocation"]));
        assert!(!contains_all_keywords(content, &["front-running", "missing"]));
    }

    #[test]
    fn test_extract_urls() {
        let content = "Check https://example.com and http://test.org for more info";
        let urls = extract_urls(content);
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://example.com".to_string()));
        assert!(urls.contains(&"http://test.org".to_string()));
    }

    #[test]
    fn test_section_exists() {
        let content = "# Title\n## Section 1\nContent\n### Subsection\nMore content";
        assert!(section_exists(content, "Title"));
        assert!(section_exists(content, "Section 1"));
        assert!(section_exists(content, "Subsection"));
        assert!(!section_exists(content, "Missing Section"));
    }
}
