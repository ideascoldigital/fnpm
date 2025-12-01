use fnpm::ast_analyzer::PackageJsonAnalyzer;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_official_pm_detection() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"packageManager": "pnpm@8.10.0"}}"#).unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    let (pm, version) = analyzer.official_package_manager().unwrap();

    assert_eq!(pm, "pnpm");
    assert_eq!(version, Some("8.10.0".to_string()));
}

#[test]
fn test_script_scanning() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{
        "scripts": {{
            "build": "pnpm run compile",
            "legacy": "npm test"
        }}
    }}"#
    )
    .unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    let scripts = analyzer.scan_scripts();

    assert_eq!(scripts.len(), 2);
    assert!(scripts
        .iter()
        .any(|(name, pm)| name == "build" && pm == "pnpm"));
    assert!(scripts
        .iter()
        .any(|(name, pm)| name == "legacy" && pm == "npm"));
}

#[test]
fn test_workspace_detection() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"workspaces": ["packages/*"]}}"#).unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    assert!(analyzer.has_workspaces());
}

#[test]
fn test_conflict_detection() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{
        "packageManager": "pnpm@8.10.0",
        "scripts": {{
            "legacy": "npm install"
        }}
    }}"#
    )
    .unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    let report = analyzer.analyze();

    assert!(!report.conflicts.is_empty());
    assert!(report.conflicts[0].contains("npm"));
    assert!(report.conflicts[0].contains("pnpm"));
}

#[test]
fn test_invalid_json() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "not valid json").unwrap();

    let result = PackageJsonAnalyzer::from_file(file.path());
    assert!(result.is_err());
}

#[test]
fn test_no_packagemanager_field() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"name": "test"}}"#).unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    assert!(analyzer.official_package_manager().is_none());
}

#[test]
fn test_dependency_counting() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{
        "dependencies": {{
            "react": "^18.0.0",
            "vue": "^3.0.0"
        }},
        "devDependencies": {{
            "typescript": "^5.0.0",
            "vitest": "^0.34.0"
        }}
    }}"#
    )
    .unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    assert_eq!(analyzer.dependency_count(), 4);
}

#[test]
fn test_engines_detection() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{
        "engines": {{
            "node": ">=18.0.0",
            "pnpm": ">=8.0.0"
        }}
    }}"#
    )
    .unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    let engines = analyzer.get_engines().unwrap();

    assert_eq!(engines.get("node"), Some(&">=18.0.0".to_string()));
    assert_eq!(engines.get("pnpm"), Some(&">=8.0.0".to_string()));
}

#[test]
fn test_multiple_conflicts() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{
        "packageManager": "pnpm@8.10.0",
        "scripts": {{
            "legacy-deploy": "npm run deploy",
            "old-ci": "yarn install && yarn test",
            "build": "tsc"
        }}
    }}"#
    )
    .unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    let report = analyzer.analyze();

    // Should detect 2 conflicts (npm and yarn, but not tsc)
    assert_eq!(report.conflicts.len(), 2);
    assert!(report.conflicts.iter().any(|c| c.contains("npm")));
    assert!(report.conflicts.iter().any(|c| c.contains("yarn")));
}

#[test]
fn test_monorepo_with_official_pm() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"{{
        "name": "monorepo-root",
        "packageManager": "pnpm@8.10.0",
        "workspaces": ["packages/*"],
        "scripts": {{
            "build": "turbo run build"
        }}
    }}"#
    )
    .unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    let report = analyzer.analyze();

    assert!(report.has_workspaces);
    assert_eq!(
        report.official_pm,
        Some(("pnpm".to_string(), Some("8.10.0".to_string())))
    );
    assert!(report.conflicts.is_empty()); // No conflicts
}

#[test]
fn test_drama_score_calculation() {
    // No official PM
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"name": "test"}}"#).unwrap();
    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    let report = analyzer.analyze();
    assert_eq!(report.drama_score(), 10); // 10 for no official PM

    // With conflicts
    let mut file2 = NamedTempFile::new().unwrap();
    writeln!(
        file2,
        r#"{{
        "packageManager": "pnpm@8.10.0",
        "scripts": {{
            "legacy": "npm install",
            "old": "yarn build"
        }}
    }}"#
    )
    .unwrap();
    let analyzer2 = PackageJsonAnalyzer::from_file(file2.path()).unwrap();
    let report2 = analyzer2.analyze();
    assert_eq!(report2.drama_score(), 30); // 15 * 2 conflicts
}

#[test]
fn test_version_without_at_sign() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"packageManager": "npm"}}"#).unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    let (pm, version) = analyzer.official_package_manager().unwrap();

    assert_eq!(pm, "npm");
    assert_eq!(version, None); // No version specified
}

#[test]
fn test_empty_package_json() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{}}"#).unwrap();

    let analyzer = PackageJsonAnalyzer::from_file(file.path()).unwrap();
    let report = analyzer.analyze();

    assert!(report.official_pm.is_none());
    assert_eq!(report.dependency_count, 0);
    assert!(!report.has_workspaces);
    assert!(report.conflicts.is_empty());
}
