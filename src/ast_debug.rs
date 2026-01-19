use anyhow::Result;
use colored::*;
use std::fs;
use std::path::Path;

use crate::ast_security_analyzer;
use crate::security;

pub fn execute_ast_debug(file: String, verbose: bool) -> Result<()> {
    println!("{}", "🔍 AST Security Analysis".bright_cyan().bold());
    println!("{}", "═══════════════════════════════════════════".bright_blue());
    println!();

    let file_path = Path::new(&file);
    
    if !file_path.exists() {
        return Err(anyhow::anyhow!("File not found: {}", file));
    }

    println!("{} {}", "📄 File:".bright_cyan(), file.bright_white());
    
    // Read the file content
    let source_code = fs::read_to_string(file_path)?;
    let line_count = source_code.lines().count();
    println!("{} {} lines", "📊 Size:".bright_cyan(), line_count);
    println!();

    // Analyze with AST
    println!("{}", "🌳 AST Analysis Results:".bright_yellow().bold());
    println!("{}", "─────────────────────────────────────────".yellow());
    
    match ast_security_analyzer::analyze_js_file(file_path) {
        Ok(issues) => {
            if issues.is_empty() {
                println!("{}", "✅ No security issues detected!".green().bold());
            } else {
                println!("{} {} {}", 
                    "⚠️".red(), 
                    "Found".red().bold(), 
                    format!("{} security issue(s)", issues.len()).red().bold()
                );
                println!();
                
                for (idx, issue) in issues.iter().enumerate() {
                    println!("{} {}", 
                        format!("Issue #{}:", idx + 1).bright_white().bold(),
                        match issue.severity {
                            security::IssueSeverity::Critical => "🔴 CRITICAL".red().bold(),
                            security::IssueSeverity::Warning => "⚠️  WARNING".yellow().bold(),
                            security::IssueSeverity::Info => "ℹ️  INFO".blue().bold(),
                        }
                    );
                    println!("  {} {}", "Type:".bright_cyan(), issue.issue_type.bright_white());
                    println!("  {} Line {}", "Location:".bright_cyan(), issue.line_number);
                    println!("  {} {}", "Description:".bright_cyan(), issue.description);
                    
                    if let Some(snippet) = &issue.code_snippet {
                        println!("  {} {}", "Code:".bright_cyan(), snippet.bright_black());
                    }
                    println!();
                }
            }
            
            if verbose {
                println!();
                println!("{}", "📋 Detailed Analysis:".bright_cyan().bold());
                println!("{}", "─────────────────────────────────────────".cyan());
                println!("  • AST parsing: {}", "✅ Success".green());
                println!("  • Source type: {}", 
                    if file.ends_with(".ts") || file.ends_with(".tsx") { "TypeScript" }
                    else if file.ends_with(".jsx") { "JSX" }
                    else if file.ends_with(".mjs") { "ES Module" }
                    else if file.ends_with(".cjs") { "CommonJS" }
                    else { "JavaScript" }
                );
                println!("  • Total lines scanned: {}", line_count);
                println!("  • Issues found: {}", issues.len());
            }
        }
        Err(e) => {
            println!("{}", "❌ AST Analysis Failed".red().bold());
            println!("  {} {}", "Error:".bright_red(), e);
            println!();
            println!("{}", "💡 This might be due to:".yellow());
            println!("  • Syntax errors in the file");
            println!("  • Minified/obfuscated code");
            println!("  • Unsupported JavaScript features");
            println!();
            println!("{}", "  Falling back to regex-based analysis...".bright_black());
        }
    }

    println!();
    println!("{}", "═══════════════════════════════════════════".bright_blue());
    
    Ok(())
}
