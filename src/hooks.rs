use anyhow::Result;
use colored::*;
use std::env;
use std::fs;
use std::path::Path;

/// Hook system for intercepting package manager commands
pub struct HookManager {
    package_manager: String,
    project_root: String,
}

impl HookManager {
    pub fn new(package_manager: String) -> Result<Self> {
        let project_root = env::current_dir()?.to_string_lossy().to_string();

        Ok(Self {
            package_manager,
            project_root,
        })
    }

    /// Create hooks for the selected package manager
    pub fn create_hooks(&self) -> Result<()> {
        self.create_fnpm_directory()?;

        // Create different types of hooks based on the platform
        if cfg!(windows) {
            self.create_windows_hooks()?;
        } else {
            self.create_unix_hooks()?;
        }

        self.create_shell_integration()?;
        self.display_setup_instructions()?;

        Ok(())
    }

    /// Remove all hooks for the current package manager
    pub fn remove_hooks(&self) -> Result<()> {
        let fnpm_dir = Path::new(".fnpm");
        if fnpm_dir.exists() {
            fs::remove_dir_all(fnpm_dir)?;
            println!("{}", "🗑️  FNPM hooks removed".yellow());
        }
        Ok(())
    }

    fn create_fnpm_directory(&self) -> Result<()> {
        fs::create_dir_all(".fnpm")?;
        Ok(())
    }

    fn create_unix_hooks(&self) -> Result<()> {
        // Create executable script that intercepts package manager commands
        let hook_script = self.generate_unix_hook_script();
        let script_path = format!(".fnpm/{}", self.package_manager);

        fs::write(&script_path, hook_script)?;

        // Make script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)?;
        }

        // Create shell aliases
        let aliases = self.generate_shell_aliases();
        fs::write(".fnpm/aliases.sh", aliases)?;

        Ok(())
    }

    fn create_windows_hooks(&self) -> Result<()> {
        // Create batch file for Windows
        let batch_script = self.generate_windows_batch_script();
        let batch_path = format!(".fnpm/{}.bat", self.package_manager);
        fs::write(&batch_path, batch_script)?;

        // Create PowerShell script as alternative
        let ps_script = self.generate_powershell_script();
        let ps_path = format!(".fnpm/{}.ps1", self.package_manager);
        fs::write(&ps_path, ps_script)?;

        Ok(())
    }

    fn generate_unix_hook_script(&self) -> String {
        let fnpm_path = self.get_fnpm_executable_path();

        format!(
            r#"#!/bin/bash
# FNPM Hook for {package_manager}
# This script intercepts {package_manager} commands and redirects them to fnpm

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Check if we're in a directory with .fnpm configuration
if [ ! -f "$PROJECT_ROOT/.fnpm/config.json" ]; then
    echo "❌ No FNPM configuration found. Run 'fnpm setup' first." >&2
    exit 1
fi

# Map common package manager commands to fnpm equivalents
case "$1" in
    "install"|"i")
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} install "$@"
        ;;
    "add"|"a")
        shift  
        FNPM_BYPASS_CLI=1 exec {fnpm_path} add "$@"
        ;;
    "remove"|"rm"|"uninstall")
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} remove "$@"
        ;;
    "run"|"r")
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} run "$@"
        ;;
    "list"|"ls")
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} list "$@"
        ;;
    "update"|"up"|"upgrade")
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} update "$@"
        ;;
    "cache")
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} cache "$@"
        ;;
    "clean")
        FNPM_BYPASS_CLI=1 exec {fnpm_path} clean
        ;;
    "--help"|"-h"|"help")
        echo "🔄 This {package_manager} command is intercepted by FNPM"
        echo "Available commands:"
        echo "  install, add, remove, run, list, update, cache, clean"
        echo ""
        echo "Use 'fnpm --help' for more information"
        ;;
    *)
        echo "🤬 Command '$1' not supported through FNPM hook" >&2
        echo "Use 'fnpm --help' to see available commands" >&2
        echo "To bypass this hook, use the full path: $(which {package_manager}) $@" >&2
        exit 1
        ;;
esac
"#,
            package_manager = self.package_manager,
            fnpm_path = fnpm_path
        )
    }

    fn generate_windows_batch_script(&self) -> String {
        let fnpm_path = self.get_fnpm_executable_path();

        format!(
            r#"@echo off
REM FNPM Hook for {package_manager}
REM This script intercepts {package_manager} commands and redirects them to fnpm

if not exist ".fnpm\config.json" (
    echo ❌ No FNPM configuration found. Run 'fnpm setup' first. >&2
    exit /b 1
)

if "%1"=="install" goto :install
if "%1"=="i" goto :install
if "%1"=="add" goto :add
if "%1"=="a" goto :add
if "%1"=="remove" goto :remove
if "%1"=="rm" goto :remove
if "%1"=="uninstall" goto :remove
if "%1"=="run" goto :run
if "%1"=="r" goto :run
if "%1"=="list" goto :list
if "%1"=="ls" goto :list
if "%1"=="update" goto :update
if "%1"=="up" goto :update
if "%1"=="upgrade" goto :update
if "%1"=="cache" goto :cache
if "%1"=="clean" goto :clean
if "%1"=="--help" goto :help
if "%1"=="-h" goto :help
if "%1"=="help" goto :help

echo 🤬 Command '%1' not supported through FNPM hook >&2
echo Use 'fnpm --help' to see available commands >&2
exit /b 1

:install
shift
{fnpm_path} install %*
goto :eof

:add
shift
{fnpm_path} add %*
goto :eof

:remove
shift
{fnpm_path} remove %*
goto :eof

:run
shift
{fnpm_path} run %*
goto :eof

:list
shift
{fnpm_path} list %*
goto :eof

:update
shift
{fnpm_path} update %*
goto :eof

:cache
shift
{fnpm_path} cache %*
goto :eof

:clean
{fnpm_path} clean
goto :eof

:help
echo 🔄 This {package_manager} command is intercepted by FNPM
echo Available commands:
echo   install, add, remove, run, list, update, cache, clean
echo.
echo Use 'fnpm --help' for more information
goto :eof
"#,
            package_manager = self.package_manager,
            fnpm_path = fnpm_path
        )
    }

    fn generate_powershell_script(&self) -> String {
        let fnpm_path = self.get_fnpm_executable_path();

        format!(
            r#"# FNPM Hook for {package_manager}
# This script intercepts {package_manager} commands and redirects them to fnpm

param(
    [Parameter(ValueFromRemainingArguments)]
    [string[]]$Arguments
)

if (-not (Test-Path ".fnpm\config.json")) {{
    Write-Error "❌ No FNPM configuration found. Run 'fnpm setup' first."
    exit 1
}}

$command = $Arguments[0]
$restArgs = $Arguments[1..($Arguments.Length-1)]

switch ($command) {{
    {{ $_ -in @("install", "i") }} {{
        & "{fnpm_path}" install @restArgs
    }}
    {{ $_ -in @("add", "a") }} {{
        & "{fnpm_path}" add @restArgs  
    }}
    {{ $_ -in @("remove", "rm", "uninstall") }} {{
        & "{fnpm_path}" remove @restArgs
    }}
    {{ $_ -in @("run", "r") }} {{
        & "{fnpm_path}" run @restArgs
    }}
    {{ $_ -in @("list", "ls") }} {{
        & "{fnpm_path}" list @restArgs
    }}
    {{ $_ -in @("update", "up", "upgrade") }} {{
        & "{fnpm_path}" update @restArgs
    }}
    "cache" {{
        & "{fnpm_path}" cache @restArgs
    }}
    "clean" {{
        & "{fnpm_path}" clean
    }}
    {{ $_ -in @("--help", "-h", "help") }} {{
        Write-Host "🔄 This {package_manager} command is intercepted by FNPM"
        Write-Host "Available commands:"
        Write-Host "  install, add, remove, run, list, update, cache, clean"
        Write-Host ""
        Write-Host "Use 'fnpm --help' for more information"
    }}
    default {{
        Write-Error "🤬 Command '$command' not supported through FNPM hook"
        Write-Error "Use 'fnpm --help' to see available commands"
        exit 1
    }}
}}
"#,
            package_manager = self.package_manager,
            fnpm_path = fnpm_path
        )
    }

    fn generate_shell_aliases(&self) -> String {
        let _project_root = &self.project_root;

        format!(
            r#"#!/bin/bash
# FNPM Shell Integration
# Source this file to enable {package_manager} command interception

# Function to intercept {package_manager} commands
{package_manager}() {{
    local fnpm_script=".fnpm/{package_manager}"
    
    # Check if we're in a directory with FNPM configuration
    if [ -f ".fnpm/config.json" ] && [ -x "$fnpm_script" ]; then
        "$fnpm_script" "$@"
    else
        # Fallback to original package manager with warning
        echo "⚠️  Using {package_manager} directly. Consider running 'fnpm setup' for better team consistency." >&2
        command {package_manager} "$@"
    fi
}}

# Export the function so it's available in subshells (bash only)
# For zsh compatibility, the function is defined globally
if [ -n "$BASH_VERSION" ]; then
    export -f {package_manager}
fi

# Auto-load FNPM hooks when entering directories with .fnpm configuration
_fnpm_cd_hook() {{
    if [ -f ".fnpm/config.json" ] && [ -f ".fnpm/aliases.sh" ]; then
        echo "🔒 FNPM hooks active - {package_manager} commands will be intercepted"
    fi
}}

# Override cd to check for FNPM configuration
cd() {{
    builtin cd "$@"
    _fnpm_cd_hook
}}

# Check current directory on source
_fnpm_cd_hook
"#,
            package_manager = self.package_manager
        )
    }

    fn create_shell_integration(&self) -> Result<()> {
        // Create a setup script that users can source
        let setup_script = format!(
            r#"#!/bin/bash
# FNPM Shell Integration Setup
# Run: source .fnpm/setup.sh

# Add .fnpm directory to PATH so our hooks take precedence
export PATH=".fnpm:$PATH"

# Source aliases if they exist
if [ -f ".fnpm/aliases.sh" ]; then
    source .fnpm/aliases.sh
fi

echo "✅ FNPM hooks activated for {package_manager}"
echo "💡 Add 'source .fnpm/setup.sh' to your shell profile for permanent activation"
"#,
            package_manager = self.package_manager
        );

        fs::write(".fnpm/setup.sh", setup_script)?;
        Ok(())
    }

    fn get_fnpm_executable_path(&self) -> String {
        // For development, prefer local build over installed version
        let dev_paths = [
            "./target/release/fnpm",
            "./target/debug/fnpm",
            "../target/release/fnpm",
            "../target/debug/fnpm",
        ];

        for path in &dev_paths {
            if Path::new(path).exists() {
                if let Ok(abs_path) = std::fs::canonicalize(path) {
                    return abs_path.to_string_lossy().to_string();
                }
            }
        }

        // Try to find fnpm in PATH as fallback
        let which_cmd = if cfg!(windows) { "where" } else { "which" };
        if let Ok(output) = std::process::Command::new(which_cmd).arg("fnpm").output() {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }

        // Final fallback
        "fnpm".to_string()
    }

    fn display_setup_instructions(&self) -> Result<()> {
        println!("\n{}", "🎉 FNPM hooks created successfully!".green().bold());
        println!("\n{}", "Setup Instructions:".yellow().bold());

        if cfg!(windows) {
            println!("{}:", "Windows".cyan().bold());
            println!("  1. Add .fnpm directory to your PATH:");
            println!(
                "     {}",
                format!("set PATH={}/.fnpm;%PATH%", self.project_root).bright_white()
            );
            println!("  2. Or run PowerShell script:");
            println!("     {}", ".fnpm/setup.ps1".bright_white());
        } else {
            println!("{}:", "Unix/Linux/macOS".cyan().bold());
            println!("  1. Source the setup script:");
            println!("     {}", "source .fnpm/setup.sh".bright_white());
            println!("  2. Or add to your shell profile (~/.bashrc, ~/.zshrc):");
            println!(
                "     {}",
                "echo 'eval \"$(fnpm source)\"' >> ~/.zshrc".bright_white()
            );
        }

        println!("\n{}", "Usage:".yellow().bold());
        println!(
            "  • Now you can use {} directly:",
            self.package_manager.cyan()
        );
        println!(
            "    {} {} some-package",
            self.package_manager.bright_white(),
            "add".bright_white()
        );
        println!(
            "    {} {}",
            self.package_manager.bright_white(),
            "install".bright_white()
        );
        println!(
            "    {} {} my-script",
            self.package_manager.bright_white(),
            "run".bright_white()
        );
        println!("  • Commands will be automatically redirected to fnpm");

        println!("\n{}", "Note:".yellow().bold());
        println!("  • Hooks only work in directories with .fnpm configuration");
        println!(
            "  • Use full path to bypass: {} {}",
            format!("$(which {})", self.package_manager).bright_white(),
            "command".bright_white()
        );

        Ok(())
    }
}
