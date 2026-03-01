// AST-based security analysis using oxc
// Provides precise detection of security issues in JavaScript/TypeScript code

use anyhow::{Context, Result};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::walk;
use oxc_ast_visit::Visit;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;
use std::collections::HashMap;
use std::path::Path;

use crate::security::{IssueSeverity, SourceCodeIssue};

/// Tracks the inferred type of a variable for security analysis
#[derive(Debug, Clone, PartialEq)]
enum VarKind {
    /// Variable holds a RegExp value (literal, new RegExp, RegExp.prototype, etc.)
    Regex,
    /// Variable holds a child_process module reference
    ChildProcess,
}

/// Security-focused AST visitor
pub struct SecurityVisitor<'a> {
    pub issues: Vec<SourceCodeIssue>,
    pub filepath: String,
    source_text: &'a str,
    /// Symbol table: tracks variable names to their inferred type
    tracked_vars: HashMap<String, VarKind>,
}

impl<'a> SecurityVisitor<'a> {
    pub fn new(filepath: String, source_text: &'a str) -> Self {
        Self {
            issues: Vec::new(),
            filepath,
            source_text,
            tracked_vars: HashMap::new(),
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

    /// Classify an expression as producing a regex value
    fn expr_is_regex(&self, expr: &Expression<'a>) -> bool {
        match expr {
            // /pattern/
            Expression::RegExpLiteral(_) => true,

            // new RegExp(...)
            Expression::NewExpression(new_expr) => {
                if let Expression::Identifier(ident) = &new_expr.callee {
                    ident.name == "RegExp"
                } else {
                    false
                }
            }

            // RegExp(...) call without new
            Expression::CallExpression(call_expr) => {
                if let Expression::Identifier(ident) = &call_expr.callee {
                    if ident.name == "RegExp" {
                        return true;
                    }
                }
                // Methods like String.prototype.match return regex-like objects
                if let Some(MemberExpression::StaticMemberExpression(static_member)) =
                    call_expr.callee.as_member_expression()
                {
                    return static_member.property.name == "match";
                }
                false
            }

            // RegExp.prototype or SomethingRegExp.prototype
            _ => {
                if let Some(MemberExpression::StaticMemberExpression(static_member)) =
                    expr.as_member_expression()
                {
                    if static_member.property.name == "prototype" {
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

    /// Check if an expression is in a RegExp context (safe to call .exec() on)
    fn is_regex_context(&self, expr: &Expression<'a>) -> bool {
        // Direct expression check (literal, new RegExp, etc.)
        if self.expr_is_regex(expr) {
            return true;
        }

        // Identifier: consult symbol table first, then fall back to name heuristic
        if let Expression::Identifier(ident) = expr {
            // Authoritative: symbol table says it's a regex
            if self.tracked_vars.get(ident.name.as_str()) == Some(&VarKind::Regex) {
                return true;
            }
            // Authoritative: symbol table says it's child_process → NOT regex
            if self.tracked_vars.get(ident.name.as_str()) == Some(&VarKind::ChildProcess) {
                return false;
            }
            // Fallback: name-based heuristic for variables not in the table
            let name_lower = ident.name.to_lowercase();
            return name_lower.contains("regex")
                || name_lower.contains("regexp")
                || name_lower.contains("pattern")
                || name_lower.contains("match")
                || name_lower.ends_with("re");
        }

        false
    }

    /// Check if a require() call imports child_process
    fn is_child_process_require(call: &CallExpression<'a>) -> bool {
        if let Expression::Identifier(ident) = &call.callee {
            if ident.name == "require" {
                if let Some(Argument::StringLiteral(lit)) = call.arguments.first() {
                    return lit.value == "child_process";
                }
            }
        }
        false
    }

    /// Extract the binding name from a simple declarator (e.g. `const x = ...`)
    fn binding_name(decl: &VariableDeclarator<'a>) -> Option<String> {
        if let BindingPattern::BindingIdentifier(binding_ident) = &decl.id {
            Some(binding_ident.name.to_string())
        } else {
            None
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

    // Track variable declarations and detect child_process imports
    fn visit_variable_declarator(&mut self, decl: &VariableDeclarator<'a>) {
        if let Some(init) = &decl.init {
            // --- Track regex values ---
            if self.expr_is_regex(init) {
                if let Some(name) = Self::binding_name(decl) {
                    self.tracked_vars.insert(name, VarKind::Regex);
                }
            }

            // --- Track child_process require ---
            if let Expression::CallExpression(call) = init {
                if Self::is_child_process_require(call) {
                    let line = self.get_line_number(decl.span.start);

                    // Simple binding: const cp = require('child_process')
                    if let Some(name) = Self::binding_name(decl) {
                        self.tracked_vars.insert(name, VarKind::ChildProcess);
                    }

                    // Destructured binding: const {exec, spawn} = require('child_process')
                    if let BindingPattern::ObjectPattern(obj_pat) = &decl.id {
                        for prop in &obj_pat.properties {
                            if let BindingPattern::BindingIdentifier(binding_ident) = &prop.value {
                                self.tracked_vars
                                    .insert(binding_ident.name.to_string(), VarKind::ChildProcess);
                            }
                        }
                    }

                    self.add_issue(
                        line,
                        "child_process_import".to_string(),
                        "child_process module imported - can execute system commands".to_string(),
                        IssueSeverity::Warning,
                        Some("require('child_process')".to_string()),
                    );
                }
            }
        }

        walk::walk_variable_declarator(self, decl);
    }

    // Track reassignments: x = /pattern/ or x = new RegExp(...)
    fn visit_assignment_expression(&mut self, expr: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(ident) = &expr.left {
            if self.expr_is_regex(&expr.right) {
                self.tracked_vars
                    .insert(ident.name.to_string(), VarKind::Regex);
            }
        }

        walk::walk_assignment_expression(self, expr);
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

    #[test]
    fn test_arbitrary_name_regex_literal_safe() {
        // Key case: variable name has NO regex-related hint, but is assigned a regex literal
        let code = r#"
            const x = /foo/;
            x.exec(str);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            !issues.iter().any(|i| i.issue_type == "command_execution"),
            "Variable assigned regex literal should not be flagged even with arbitrary name"
        );
    }

    #[test]
    fn test_arbitrary_name_new_regexp_safe() {
        // Variable with no regex hint, assigned new RegExp(...)
        let code = r#"
            const checker = new RegExp('\\d+');
            const result = checker.exec(input);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            !issues.iter().any(|i| i.issue_type == "command_execution"),
            "Variable assigned new RegExp() should not be flagged"
        );
    }

    #[test]
    fn test_arbitrary_name_regexp_call_safe() {
        // RegExp() without new keyword
        let code = r#"
            var o = RegExp('test', 'g');
            o.exec(str);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            !issues.iter().any(|i| i.issue_type == "command_execution"),
            "Variable assigned RegExp() call should not be flagged"
        );
    }

    #[test]
    fn test_reassigned_regex_safe() {
        // Variable reassigned to regex after initial declaration
        let code = r#"
            let validator;
            validator = /^[a-z]+$/i;
            validator.exec(input);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            !issues.iter().any(|i| i.issue_type == "command_execution"),
            "Variable reassigned to regex literal should not be flagged"
        );
    }

    #[test]
    fn test_minified_regex_variable_safe() {
        // Minified code: single-letter variable assigned regex
        let code = r#"
            var a = /x/;
            a.exec(b);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            !issues.iter().any(|i| i.issue_type == "command_execution"),
            "Minified regex variable should not be flagged"
        );
    }

    #[test]
    fn test_destructured_child_process_flagged() {
        // Destructured import of child_process should be flagged
        let code = r#"
            const {exec, spawn} = require('child_process');
            exec('ls');
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            issues
                .iter()
                .any(|i| i.issue_type == "child_process_import"),
            "Destructured child_process import should be flagged"
        );
    }

    #[test]
    fn test_child_process_tracked_not_regex() {
        // child_process variable must NOT be treated as regex
        let code = r#"
            const cp = require('child_process');
            cp.exec('rm -rf /');
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            issues.iter().any(|i| i.issue_type == "command_execution"),
            "child_process.exec() must be flagged as command execution"
        );
    }

    #[test]
    fn test_multiple_regex_vars_safe() {
        // Multiple variables assigned different regex forms
        let code = r#"
            const a = /first/;
            const b = new RegExp('second');
            const c = RegExp('third');
            a.exec(str);
            b.exec(str);
            c.exec(str);
        "#;

        let issues = analyze_js_source(code, "test.js".to_string()).unwrap();
        assert!(
            !issues.iter().any(|i| i.issue_type == "command_execution"),
            "Multiple regex variables should all be safe"
        );
    }
}
