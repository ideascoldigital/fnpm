// AST-based security analysis using oxc
// Provides precise detection of security issues in JavaScript/TypeScript code

use anyhow::{Context, Result};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::visit::walk;
use oxc_ast::Visit;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;
use std::collections::HashMap;
use std::path::Path;

use crate::security::{IssueSeverity, SourceCodeIssue};

/// Security-focused AST visitor
pub struct SecurityVisitor<'a> {
    pub issues: Vec<SourceCodeIssue>,
    pub filepath: String,
    source_text: &'a str,
    /// Track variable assignments to RegExp.prototype or similar
    regex_prototype_vars: HashMap<String, bool>,
}

impl<'a> SecurityVisitor<'a> {
    pub fn new(filepath: String, source_text: &'a str) -> Self {
        Self {
            issues: Vec::new(),
            filepath,
            source_text,
            regex_prototype_vars: HashMap::new(),
        }
    }

    fn add_issue(
        &mut self,
        line_number: usize,
        issue_type: String,
        description: String,
        severity: IssueSeverity,
        code_snippet: Option<String>,
    ) {
        self.issues.push(SourceCodeIssue {
            file_path: self.filepath.clone(),
            line_number,
            issue_type,
            description,
            severity,
            code_snippet,
        });
    }

    fn get_line_number(&self, offset: u32) -> usize {
        self.source_text[..offset as usize]
            .chars()
            .filter(|&c| c == '\n')
            .count()
            + 1
    }

    fn get_code_snippet(&self, offset: u32, length: u32) -> String {
        let start = offset as usize;
        let end = (offset + length) as usize;
        self.source_text.get(start..end).unwrap_or("").to_string()
    }

    /// Check if an expression is in a RegExp context
    fn is_regex_context(&self, expr: &Expression<'a>) -> bool {
        match expr {
            // Direct regex literal: /pattern/.exec()
            Expression::RegExpLiteral(_) => true,

            // Identifier that might be a regex variable
            Expression::Identifier(ident) => {
                // First check if we tracked this variable as RegExp.prototype
                if self.regex_prototype_vars.contains_key(ident.name.as_str()) {
                    return true;
                }

                // Check if the identifier name suggests it's a regex
                let name_lower = ident.name.to_lowercase();
                name_lower.contains("regex")
                    || name_lower.contains("regexp")
                    || name_lower.contains("pattern")
                    || name_lower.contains("match")
                    || name_lower.ends_with("re")
            }

            // new RegExp().exec()
            Expression::NewExpression(new_expr) => {
                if let Expression::Identifier(ident) = &new_expr.callee {
                    ident.name == "RegExp"
                } else {
                    false
                }
            }

            // Method call that returns a regex
            Expression::CallExpression(call_expr) => {
                // Check if it's a method that typically returns regex
                if let Some(MemberExpression::StaticMemberExpression(static_member)) =
                    call_expr.callee.as_member_expression()
                {
                    // Methods like String.prototype.match return regex-like objects
                    return static_member.property.name == "match";
                }
                false
            }

            // Member expression accessing .prototype (e.g., RegExp.prototype, BabelRegExp.prototype)
            // Or any other expression type
            _ => {
                // Try to get as member expression
                if let Some(MemberExpression::StaticMemberExpression(static_member)) =
                    expr.as_member_expression()
                {
                    // Check if accessing .prototype property
                    if static_member.property.name == "prototype" {
                        // Check if the object is RegExp-related
                        if let Expression::Identifier(ident) = &static_member.object {
                            let name_lower = ident.name.to_lowercase();
                            return name_lower.contains("regexp") || name_lower.contains("regex");
                        }
                    }
                }
                false
            }
        }
    }
}

impl<'a> Visit<'a> for SecurityVisitor<'a> {
    // Detect command execution (but not RegExp.exec)
    fn visit_member_expression(&mut self, expr: &MemberExpression<'a>) {
        if let MemberExpression::StaticMemberExpression(static_expr) = expr {
            let property_name = static_expr.property.name.as_str();

            // Check for dangerous methods
            let dangerous_methods = ["exec", "execSync", "spawn", "spawnSync"];
            if dangerous_methods.contains(&property_name) {
                // Check if this is a RegExp.exec() call (safe) vs child_process.exec() (dangerous)
                let is_regex_exec = self.is_regex_context(&static_expr.object);

                if !is_regex_exec {
                    let line = self.get_line_number(static_expr.span.start);
                    let snippet =
                        self.get_code_snippet(static_expr.span.start, static_expr.span.size());

                    self.add_issue(
                        line,
                        "command_execution".to_string(),
                        format!("Command execution method '{}' detected", property_name),
                        IssueSeverity::Critical,
                        Some(snippet),
                    );
                }
            }
        }

        walk::walk_member_expression(self, expr);
    }

    // Detect eval() calls
    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        // Check if callee is an identifier named "eval"
        if let Expression::Identifier(ident) = &expr.callee {
            if ident.name == "eval" {
                let line = self.get_line_number(expr.span.start);
                let snippet = self.get_code_snippet(expr.span.start, expr.span.size());

                self.add_issue(
                    line,
                    "eval_usage".to_string(),
                    "Direct eval() usage detected - allows arbitrary code execution".to_string(),
                    IssueSeverity::Critical,
                    Some(snippet),
                );
            }
        }

        // Continue visiting child nodes
        walk::walk_call_expression(self, expr);
    }

    // Detect dynamic require() calls
    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        // Check if the source is not a string literal (dynamic import)
        if !matches!(&expr.source, Expression::StringLiteral(_)) {
            let line = self.get_line_number(expr.span.start);
            let snippet = self.get_code_snippet(expr.span.start, expr.span.size());

            self.add_issue(
                line,
                "dynamic_import".to_string(),
                "Dynamic import with non-literal path - potential security risk".to_string(),
                IssueSeverity::Warning,
                Some(snippet),
            );
        }

        walk::walk_import_expression(self, expr);
    }

    // Detect new Function() calls
    fn visit_new_expression(&mut self, expr: &NewExpression<'a>) {
        if let Expression::Identifier(ident) = &expr.callee {
            if ident.name == "Function" {
                let line = self.get_line_number(expr.span.start);
                let snippet = self.get_code_snippet(expr.span.start, expr.span.size());

                self.add_issue(
                    line,
                    "dynamic_function".to_string(),
                    "Dynamic function creation with new Function() - potential code injection"
                        .to_string(),
                    IssueSeverity::Warning,
                    Some(snippet),
                );
            }
        }

        walk::walk_new_expression(self, expr);
    }

    // Detect child_process usage and track RegExp.prototype assignments
    fn visit_variable_declarator(&mut self, decl: &VariableDeclarator<'a>) {
        // Track if this variable is assigned to RegExp.prototype
        if let Some(init) = &decl.init {
            // Check if init is RegExp.prototype
            if let Some(MemberExpression::StaticMemberExpression(static_member)) =
                init.as_member_expression()
            {
                if static_member.property.name == "prototype" {
                    if let Expression::Identifier(ident) = &static_member.object {
                        if ident.name == "RegExp" {
                            // This variable is assigned to RegExp.prototype
                            if let BindingPatternKind::BindingIdentifier(binding_ident) =
                                &decl.id.kind
                            {
                                self.regex_prototype_vars
                                    .insert(binding_ident.name.to_string(), true);
                            }
                        }
                    }
                }
            }

            // Check for child_process require
            if let Expression::CallExpression(call) = init {
                if let Expression::Identifier(ident) = &call.callee {
                    if ident.name == "require" {
                        if let Some(Argument::StringLiteral(lit)) = call.arguments.first() {
                            if lit.value == "child_process" {
                                let line = self.get_line_number(decl.span.start);

                                self.add_issue(
                                    line,
                                    "child_process_import".to_string(),
                                    "child_process module imported - can execute system commands"
                                        .to_string(),
                                    IssueSeverity::Warning,
                                    Some(format!("require('{}')", lit.value)),
                                );
                            }
                        }
                    }
                }
            }
        }

        walk::walk_variable_declarator(self, decl);
    }
}

/// Analyze JavaScript/TypeScript file for security issues using AST
pub fn analyze_js_file(path: &Path) -> Result<Vec<SourceCodeIssue>> {
    let source_text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    analyze_js_source(&source_text, path.to_string_lossy().to_string())
}

/// Analyze JavaScript/TypeScript source code for security issues
pub fn analyze_js_source(source_text: &str, filepath: String) -> Result<Vec<SourceCodeIssue>> {
    let allocator = Allocator::default();

    // Determine source type from filepath
    let source_type = if filepath.ends_with(".ts") || filepath.ends_with(".tsx") {
        SourceType::ts()
    } else if filepath.ends_with(".jsx") {
        SourceType::jsx()
    } else if filepath.ends_with(".mjs") {
        SourceType::mjs()
    } else if filepath.ends_with(".cjs") {
        SourceType::cjs()
    } else {
        SourceType::unambiguous()
    };

    // Parse the source code
    let ParserReturn {
        program, errors, ..
    } = Parser::new(&allocator, source_text, source_type).parse();

    // If there are parse errors, fall back to regex-based analysis
    if !errors.is_empty() {
        return Ok(Vec::new()); // Return empty, let regex scanner handle it
    }

    // Create visitor and analyze
    let mut visitor = SecurityVisitor::new(filepath, source_text);
    visitor.visit_program(&program);

    Ok(visitor.issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_eval() {
        let code = r#"
            const x = eval("1 + 1");
            console.log(x);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(issues.iter().any(|i| i.issue_type == "eval_usage"));
    }

    #[test]
    fn test_detect_new_function() {
        let code = r#"
            const fn = new Function('return 1');
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(issues.iter().any(|i| i.issue_type == "dynamic_function"));
    }

    #[test]
    fn test_ignore_eval_in_string() {
        let code = r#"
            console.log("eval() is dangerous");
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(!issues.iter().any(|i| i.issue_type == "eval_usage"));
    }

    #[test]
    fn test_detect_child_process() {
        let code = r#"
            const cp = require('child_process');
            cp.exec('ls');
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(issues
            .iter()
            .any(|i| i.issue_type == "child_process_import"));
        assert!(issues.iter().any(|i| i.issue_type == "command_execution"));
    }

    #[test]
    fn test_dynamic_import() {
        let code = r#"
            const moduleName = "dangerous";
            import(moduleName);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(issues.iter().any(|i| i.issue_type == "dynamic_import"));
    }

    #[test]
    fn test_static_import_safe() {
        let code = r#"
            import("./safe-module");
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(!issues.iter().any(|i| i.issue_type == "dynamic_import"));
    }

    #[test]
    fn test_regex_exec_safe() {
        let code = r#"
            const match = /^\/(.*)\/([yugi]*)$/.exec(value);
            const pattern = /test/;
            pattern.exec(str);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(!issues.iter().any(|i| i.issue_type == "command_execution"));
    }

    #[test]
    fn test_child_process_exec_dangerous() {
        let code = r#"
            const cp = require('child_process');
            cp.exec('ls -la');
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(issues.iter().any(|i| i.issue_type == "command_execution"));
    }

    #[test]
    fn test_new_regexp_exec_safe() {
        let code = r#"
            const regex = new RegExp('pattern');
            regex.exec(str);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(!issues.iter().any(|i| i.issue_type == "command_execution"));
    }

    #[test]
    fn test_named_regexp_variable_safe() {
        let code = r#"
            var simpleEncodingRegExp = /^\s*([^\s;]+)\s*(?:;(.*))?$/;
            var match = simpleEncodingRegExp.exec(str);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            !issues.iter().any(|i| i.issue_type == "command_execution"),
            "simpleEncodingRegExp.exec() should not be flagged as command execution"
        );
    }

    #[test]
    fn test_various_regex_names_safe() {
        let code = r#"
            const myPattern = /test/;
            const urlMatch = /url/;
            const testRe = /re/;
            
            myPattern.exec(str);
            urlMatch.exec(str);
            testRe.exec(str);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            !issues.iter().any(|i| i.issue_type == "command_execution"),
            "Variables with regex-related names should not be flagged"
        );
    }

    #[test]
    fn test_regexp_prototype_variable_safe() {
        // Babel case: var e = RegExp.prototype; e.exec.call(...)
        let code = r#"
            function test() {
                var e = RegExp.prototype;
                var result = e.exec.call(this, str);
                return result;
            }
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            !issues.iter().any(|i| i.issue_type == "command_execution"),
            "RegExp.prototype variable should not be flagged as command execution"
        );
    }

    #[test]
    fn test_babel_regexp_wrapper_safe() {
        // Real Babel wrapRegExp helper case
        let code = r#"
            function _wrapRegExp() {
                var e = RegExp.prototype;
                function BabelRegExp(e, t, p) {
                    var o = RegExp(e, t);
                    return o;
                }
                BabelRegExp.prototype.exec = function (r) {
                    var t = e.exec.call(this, r);
                    return t;
                };
                return BabelRegExp;
            }
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            !issues.iter().any(|i| i.issue_type == "command_execution"),
            "Babel RegExp wrapper should not be flagged"
        );
    }
}
