use std::fs;
use std::path::Path;
use doc_tests::{extract_section, section_exists};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_md_exists() {
        assert!(
            Path::new("../SECURITY.md").exists(),
            "SECURITY.md file must exist at repository root"
        );
    }

    #[test]
    fn test_required_sections_present() {
        let content = fs::read_to_string("../SECURITY.md")
            .expect("Failed to read SECURITY.md");

        // Check for main sections
        assert!(
            section_exists(&content, "Overview"),
            "SECURITY.md must contain Overview section"
        );
        assert!(
            section_exists(&content, "Known Limitations"),
            "SECURITY.md must contain Known Limitations section"
        );
        assert!(
            section_exists(&content, "Operational Security Guidance"),
            "SECURITY.md must contain Operational Security Guidance section"
        );
        assert!(
            section_exists(&content, "References"),
            "SECURITY.md must contain References section"
        );

        // Check for Revocation Front-Running subsection
        assert!(
            section_exists(&content, "Revocation Front-Running"),
            "SECURITY.md must contain Revocation Front-Running subsection"
        );
    }

    #[test]
    fn test_attack_vector_section_structure() {
        let content = fs::read_to_string("../SECURITY.md")
            .expect("Failed to read SECURITY.md");

        let attack_section = extract_section(&content, "Revocation Front-Running");
        
        // Check for required subsections within the attack vector section
        assert!(
            section_exists(&attack_section, "Attack Description"),
            "Revocation Front-Running section must contain Attack Description subsection"
        );
        assert!(
            section_exists(&attack_section, "Technical Background"),
            "Revocation Front-Running section must contain Technical Background subsection"
        );
        assert!(
            section_exists(&attack_section, "Risk Assessment"),
            "Revocation Front-Running section must contain Risk Assessment subsection"
        );
        assert!(
            section_exists(&attack_section, "Mitigation Strategies"),
            "Revocation Front-Running section must contain Mitigation Strategies subsection"
        );
    }

    #[test]
    fn test_operational_guidance_structure() {
        let content = fs::read_to_string("../SECURITY.md")
            .expect("Failed to read SECURITY.md");

        let guidance_section = extract_section(&content, "Operational Security Guidance");
        
        // Check for required subsections
        assert!(
            section_exists(&guidance_section, "Safe Revocation Procedures"),
            "Operational Security Guidance must contain Safe Revocation Procedures subsection"
        );
        assert!(
            section_exists(&guidance_section, "Monitoring Recommendations"),
            "Operational Security Guidance must contain Monitoring Recommendations subsection"
        );
        assert!(
            section_exists(&guidance_section, "Emergency Response"),
            "Operational Security Guidance must contain Emergency Response subsection"
        );
    }
}
