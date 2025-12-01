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
        self.create_hooks_internal(true)
    }

    /// Create hooks silently (without displaying setup instructions)
    pub fn create_hooks_silent(&self) -> Result<()> {
        self.create_hooks_internal(false)
    }

    fn create_hooks_internal(&self, show_instructions: bool) -> Result<()> {
        self.create_fnpm_directory()?;

        // Create different types of hooks based on the platform
        if cfg!(windows) {
            self.create_windows_hooks()?;
        } else {
            self.create_unix_hooks()?;
        }

        self.create_shell_integration()?;

        if show_instructions {
            self.display_setup_instructions()?;
        }

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
        let hook_content = self.generate_unix_hook_script();
        let hook_path = format!(".fnpm/{}", self.package_manager);
        std::fs::write(&hook_path, hook_content)?;

        // Make the hook executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&hook_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&hook_path, perms)?;
        }

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

# Prevent infinite loops - if this variable is set, we're already inside a hook
if [ -n "$FNPM_HOOK_ACTIVE" ]; then
    echo "❌ FNPM hook recursion detected. Please check your PATH configuration." >&2
    exit 1
fi

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
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        echo ""
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} install "$@"
        ;;
    "add"|"a")
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        echo ""
        shift
        # Parse flags and packages separately to handle yarn's --dev flag
        FLAGS=""
        PACKAGES=""
        while [ $# -gt 0 ]; do
            case "$1" in
                --dev|-D)
                    FLAGS="$FLAGS -D"
                    shift
                    ;;
                --global|-g)
                    FLAGS="$FLAGS -g"
                    shift
                    ;;
                *)
                    PACKAGES="$PACKAGES $1"
                    shift
                    ;;
            esac
        done
        FNPM_BYPASS_CLI=1 exec {fnpm_path} add $FLAGS $PACKAGES
        ;;
    "remove"|"rm"|"uninstall")
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        echo ""
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} remove "$@"
        ;;
    "run"|"r")
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        echo ""
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} run "$@"
        ;;
    "list"|"ls")
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        echo ""
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} list "$@"
        ;;
    "update"|"up"|"upgrade")
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        echo ""
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} update "$@"
        ;;
    "cache")
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        echo ""
        shift
        FNPM_BYPASS_CLI=1 exec {fnpm_path} cache "$@"
        ;;
    "clean")
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        echo ""
        FNPM_BYPASS_CLI=1 exec {fnpm_path} clean
        ;;
    "dlx")
        shift
        # Check if --help is requested for dlx
        if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
            echo "🔄 This {package_manager} dlx command is intercepted by FNPM"
            echo ""
            echo "Usage: {package_manager} dlx <command> [args...]"
            echo ""
            echo "Execute a command using the package manager's executor."
            echo "This is equivalent to npx for npm, pnpm dlx for pnpm, yarn dlx for yarn, etc."
            echo ""
            echo "Examples:"
            echo "  {package_manager} dlx create-react-app my-app"
            echo "  {package_manager} dlx typescript --version"
            echo "  {package_manager} dlx @angular/cli new my-project"
            echo ""
            echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        else
            echo ""
            echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
            echo ""
            FNPM_BYPASS_CLI=1 exec {fnpm_path} dlx "$@"
        fi
        ;;
    "x")
        # This handles 'bun x' which should redirect to 'bunx'
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        echo ""
        shift
        FNPM_BYPASS_CLI=1 exec bunx "$@"
        ;;
    "--help"|"-h"|"help")
        echo "🔄 This {package_manager} command is intercepted by FNPM"
        echo "Available commands:"
        echo "  install, add, remove, run, list, update, cache, clean, dlx, x"
        echo ""
        echo "Use 'fnpm --help' for more information"
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        ;;
    *)
        echo ""
        echo "⚠️  Command '$1' is not yet supported by FNPM hooks" >&2
        echo "📝 Help us improve! Report this command at:" >&2
        echo "   https://github.com/ideascoldigital/fnpm/issues/new?title=Add%20support%20for%20{package_manager}%20$1&body=Please%20add%20support%20for%20the%20command:%20{package_manager}%20$1" >&2
        echo ""
        echo "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        echo ""
        echo "🔄 Executing the real {package_manager} command..."
        echo ""
        
        # Set flag to prevent recursion
        export FNPM_HOOK_ACTIVE=1
        
        # Execute the real package manager command
        # Try common locations first to avoid PATH issues
        REAL_CMD=""
        
        # Common locations for package managers
        COMMON_PATHS="/usr/local/bin /opt/homebrew/bin /usr/bin $HOME/.bun/bin $HOME/.local/bin"
        
        for path in $COMMON_PATHS; do
            if [ -x "$path/{package_manager}" ]; then
                REAL_CMD="$path/{package_manager}"
                break
            fi
        done
        
        # If not found in common paths, search PATH excluding .fnpm
        if [ -z "$REAL_CMD" ]; then
            # Get the absolute path of this script's directory
            HOOK_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
            
            for path in $(echo $PATH | tr ':' '\n'); do
                # Convert to absolute path for comparison
                abs_path="$(cd "$path" 2>/dev/null && pwd)" || continue
                
                # Skip if this is the .fnpm directory
                if [ "$abs_path" = "$HOOK_DIR" ]; then
                    continue
                fi
                
                if [ -x "$path/{package_manager}" ]; then
                    REAL_CMD="$path/{package_manager}"
                    break
                fi
            done
        fi
        
        if [ -n "$REAL_CMD" ]; then
            exec "$REAL_CMD" "$@"
        else
            echo "❌ Could not find real {package_manager} command" >&2
            echo "💡 Try using the full path: /usr/local/bin/{package_manager} $@" >&2
            exit 1
        fi
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

REM Prevent infinite loops
if defined FNPM_HOOK_ACTIVE (
    echo ❌ FNPM hook recursion detected. Please check your PATH configuration. >&2
    exit /b 1
)

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
if "%1"=="dlx" goto :dlx
if "%1"=="x" goto :x
if "%1"=="--help" goto :help
if "%1"=="-h" goto :help
if "%1"=="help" goto :help

echo.
echo ⚠️  Command '%1' is not yet supported by FNPM hooks
echo 📝 Help us improve! Report this command at:
echo    https://github.com/ideascoldigital/fnpm/issues/new?title=Add%%20support%%20for%%20{package_manager}%%20%1^&body=Please%%20add%%20support%%20for%%20the%%20command:%%20{package_manager}%%20%1
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
echo 🔄 Executing the real {package_manager} command...
echo.

REM Set flag to prevent recursion
set FNPM_HOOK_ACTIVE=1

REM Find and execute the real package manager
set REAL_CMD=
set SCRIPT_DIR=%~dp0

REM Common locations for package managers
if exist "C:\Program Files\nodejs\{package_manager}.cmd" (
    set REAL_CMD=C:\Program Files\nodejs\{package_manager}.cmd
    goto :execute_real
)
if exist "%USERPROFILE%\AppData\Roaming\npm\{package_manager}.cmd" (
    set REAL_CMD=%USERPROFILE%\AppData\Roaming\npm\{package_manager}.cmd
    goto :execute_real
)
if exist "%PROGRAMFILES%\nodejs\{package_manager}.cmd" (
    set REAL_CMD=%PROGRAMFILES%\nodejs\{package_manager}.cmd
    goto :execute_real
)

REM Search PATH excluding .fnpm directory
for %%p in ("%PATH:;=" "%") do (
    if not "%%~p"=="%SCRIPT_DIR:~0,-1%" (
        if exist "%%~p\{package_manager}.exe" (
            set REAL_CMD=%%~p\{package_manager}.exe
            goto :execute_real
        )
        if exist "%%~p\{package_manager}.cmd" (
            set REAL_CMD=%%~p\{package_manager}.cmd
            goto :execute_real
        )
        if exist "%%~p\{package_manager}.bat" (
            set REAL_CMD=%%~p\{package_manager}.bat
            goto :execute_real
        )
    )
)

echo ❌ Could not find real {package_manager} command
echo 💡 Check your PATH or install {package_manager}
exit /b 1

:execute_real
"%REAL_CMD%" %*

:install
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
shift
{fnpm_path} install %*
goto :eof

:add
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
shift
REM Parse flags and packages
set FLAGS=
set PACKAGES=
:parse_add_args
if "%1"=="" goto :execute_add
if "%1"=="--dev" (
    set FLAGS=%FLAGS% -D
    shift
    goto :parse_add_args
)
if "%1"=="-D" (
    set FLAGS=%FLAGS% -D
    shift
    goto :parse_add_args
)
if "%1"=="--global" (
    set FLAGS=%FLAGS% -g
    shift
    goto :parse_add_args
)
if "%1"=="-g" (
    set FLAGS=%FLAGS% -g
    shift
    goto :parse_add_args
)
set PACKAGES=%PACKAGES% %1
shift
goto :parse_add_args
:execute_add
{fnpm_path} add %FLAGS% %PACKAGES%
goto :eof

:remove
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
shift
{fnpm_path} remove %*
goto :eof

:run
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
shift
{fnpm_path} run %*
goto :eof

:list
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
shift
{fnpm_path} list %*
goto :eof

:update
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
shift
{fnpm_path} update %*
goto :eof

:cache
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
shift
{fnpm_path} cache %*
goto :eof

:clean
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
{fnpm_path} clean
goto :eof

:x
shift
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
bunx %*
goto :eof

:dlx
shift
if "%1"=="--help" goto :dlx_help
if "%1"=="-h" goto :dlx_help
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
echo.
{fnpm_path} dlx %*
goto :eof

:dlx_help
echo 🔄 This {package_manager} dlx command is intercepted by FNPM
echo.
echo Usage: {package_manager} dlx ^<command^> [args...]
echo.
echo Execute a command using the package manager's executor.
echo This is equivalent to npx for npm, pnpm dlx for pnpm, yarn dlx for yarn, etc.
echo.
echo Examples:
echo   {package_manager} dlx create-react-app my-app
echo   {package_manager} dlx typescript --version
echo   {package_manager} dlx @angular/cli new my-project
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
goto :eof

:help
echo 🔄 This {package_manager} command is intercepted by FNPM
echo Available commands:
echo   install, add, remove, run, list, update, cache, clean, dlx, x
echo.
echo Use 'fnpm --help' for more information
echo.
echo ⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm
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

# Prevent infinite loops
if ($env:FNPM_HOOK_ACTIVE -eq "1") {{
    Write-Error "❌ FNPM hook recursion detected. Please check your PATH configuration."
    exit 1
}}

if (-not (Test-Path ".fnpm\config.json")) {{
    Write-Error "❌ No FNPM configuration found. Run 'fnpm setup' first."
    exit 1
}}

$command = $Arguments[0]
$restArgs = $Arguments[1..($Arguments.Length-1)]

switch ($command) {{
    {{ $_ -in @("install", "i") }} {{
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        Write-Host ""
        & "{fnpm_path}" install @restArgs
    }}
    {{ $_ -in @("add", "a") }} {{
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        Write-Host ""
        # Parse flags and packages separately
        $flags = @()
        $packages = @()
        foreach ($arg in $restArgs) {{
            if ($arg -eq "--dev" -or $arg -eq "-D") {{
                $flags += "-D"
            }}
            elseif ($arg -eq "--global" -or $arg -eq "-g") {{
                $flags += "-g"
            }}
            else {{
                $packages += $arg
            }}
        }}
        & "{fnpm_path}" add @flags @packages
    }}
    {{ $_ -in @("remove", "rm", "uninstall") }} {{
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        Write-Host ""
        & "{fnpm_path}" remove @restArgs
    }}
    {{ $_ -in @("run", "r") }} {{
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        Write-Host ""
        & "{fnpm_path}" run @restArgs
    }}
    {{ $_ -in @("list", "ls") }} {{
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        Write-Host ""
        & "{fnpm_path}" list @restArgs
    }}
    {{ $_ -in @("update", "up", "upgrade") }} {{
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        Write-Host ""
        & "{fnpm_path}" update @restArgs
    }}
    "cache" {{
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        Write-Host ""
        & "{fnpm_path}" cache @restArgs
    }}
    "clean" {{
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        Write-Host ""
        & "{fnpm_path}" clean
    }}
    "dlx" {{
        # Check if --help is requested for dlx
        if ($restArgs[0] -eq "--help" -or $restArgs[0] -eq "-h") {{
            Write-Host "🔄 This {package_manager} dlx command is intercepted by FNPM"
            Write-Host ""
            Write-Host "Usage: {package_manager} dlx <command> [args...]"
            Write-Host ""
            Write-Host "Execute a command using the package manager's executor."
            Write-Host "This is equivalent to npx for npm, pnpm dlx for pnpm, yarn dlx for yarn, etc."
            Write-Host ""
            Write-Host "Examples:"
            Write-Host "  {package_manager} dlx create-react-app my-app"
            Write-Host "  {package_manager} dlx typescript --version"
            Write-Host "  {package_manager} dlx @angular/cli new my-project"
            Write-Host ""
            Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        }} else {{
            Write-Host ""
            Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
            Write-Host ""
            & "{fnpm_path}" dlx @restArgs
        }}
    }}
    "x" {{
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        Write-Host ""
        & bunx @restArgs
    }}
    {{ $_ -in @("--help", "-h", "help") }} {{
        Write-Host "🔄 This {package_manager} command is intercepted by FNPM"
        Write-Host "Available commands:"
        Write-Host "  install, add, remove, run, list, update, cache, clean, dlx, x"
        Write-Host ""
        Write-Host "Use 'fnpm --help' for more information"
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
    }}
    default {{
        Write-Host ""
        Write-Host "⚠️  Command '$command' is not yet supported by FNPM hooks" -ForegroundColor Yellow
        Write-Host "📝 Help us improve! Report this command at:" -ForegroundColor Cyan
        Write-Host "   https://github.com/ideascoldigital/fnpm/issues/new?title=Add%20support%20for%20{package_manager}%20$command&body=Please%20add%20support%20for%20the%20command:%20{package_manager}%20$command" -ForegroundColor Blue
        Write-Host ""
        Write-Host "⭐ Like fnpm? Give us a star: https://github.com/ideascoldigital/fnpm"
        Write-Host ""
        Write-Host "🔄 Executing the real {package_manager} command..." -ForegroundColor Green
        Write-Host ""
        
        # Set flag to prevent recursion
        $env:FNPM_HOOK_ACTIVE = "1"
        
        # Find and execute the real package manager
        $realCmd = $null
        $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
        
        # Try common locations first
        $commonPaths = @(
            "C:\Program Files\nodejs\{package_manager}.cmd",
            "$env:USERPROFILE\AppData\Roaming\npm\{package_manager}.cmd",
            "$env:PROGRAMFILES\nodejs\{package_manager}.cmd"
        )
        
        foreach ($path in $commonPaths) {{
            if (Test-Path $path) {{
                $realCmd = $path
                break
            }}
        }}
        
        # Search PATH excluding .fnpm directory
        if (-not $realCmd) {{
            $paths = $env:PATH -split ';'
            
            foreach ($path in $paths) {{
                # Skip if this is the .fnpm directory
                if ($path -eq $scriptDir) {{
                    continue
                }}
                
                $cmdPath = Join-Path $path "{package_manager}.exe"
                if (Test-Path $cmdPath) {{
                    $realCmd = $cmdPath
                    break
                }}
                $cmdPath = Join-Path $path "{package_manager}.cmd"
                if (Test-Path $cmdPath) {{
                    $realCmd = $cmdPath
                    break
                }}
            }}
        }}
        
        if ($realCmd) {{
            & $realCmd @Arguments
        }} else {{
            Write-Error "❌ Could not find real {package_manager} command"
            Write-Host "💡 Check your PATH or install {package_manager}" -ForegroundColor Yellow
            exit 1
        }}
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
        // Create aliases file
        let aliases_content = self.generate_shell_aliases();
        fs::write(".fnpm/aliases.sh", aliases_content)?;

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

        println!();
        println!(
            "{} {} {}",
            "⭐".bright_yellow(),
            "Like fnpm? Give us a star on GitHub:".bright_white(),
            "https://github.com/ideascoldigital/fnpm"
                .bright_cyan()
                .underline()
        );

        Ok(())
    }
}
