# FNPM Hooks System

FNPM includes a powerful hooks system that allows you to seamlessly intercept package manager commands and redirect them through fnpm. This ensures team consistency while maintaining the familiar developer experience.

## Overview

When you run `fnpm setup`, hooks are automatically created that intercept direct package manager commands (like `pnpm add`, `yarn install`, etc.) and redirect them to the equivalent `fnpm` commands.

## How It Works

1. **Setup Phase**: When you run `fnpm setup <package-manager>`, FNPM creates:
   - Executable hook scripts in `.fnpm/` directory
   - Shell aliases and functions
   - Setup scripts for easy activation

2. **Interception**: When activated, direct package manager commands are intercepted and redirected:
   ```bash
   pnpm add lodash    # → fnpm add lodash
   yarn install       # → fnpm install  
   npm run build      # → fnpm run build
   ```

3. **Transparency**: Developers can use their preferred package manager commands without knowing they're being redirected through fnpm.

## Quick Start

### 1. Setup with Hooks (Default)
```bash
# Setup fnpm with automatic hook creation
fnpm setup pnpm

# This creates hooks automatically
```

### 2. Setup without Hooks
```bash
# Setup fnpm but skip hook creation
fnpm setup --no-hooks npm

# Create hooks later if needed
fnpm hooks create
```

### 3. Activate Hooks
```bash
# Activate hooks for current session
source .fnpm/setup.sh

# Or add to your shell profile for permanent activation
echo 'source .fnpm/setup.sh' >> ~/.bashrc  # or ~/.zshrc
```

### 4. Use Your Package Manager Normally
```bash
# These commands are now intercepted by fnpm
pnpm add express
pnpm install
pnpm run dev
pnpm remove lodash
```

## Hook Management Commands

### Check Hook Status
```bash
fnpm hooks status
```
Shows current hook configuration and setup instructions.

### Create/Update Hooks
```bash
fnpm hooks create
```
Creates or updates hooks for the configured package manager.

### Remove Hooks
```bash
fnpm hooks remove
```
Removes all hook files and directories.

## Platform Support

### Unix/Linux/macOS
- Creates executable shell scripts in `.fnpm/<package-manager>`
- Provides shell functions and aliases in `.fnpm/aliases.sh`
- Setup script at `.fnpm/setup.sh`

### Windows
- Creates batch files (`.fnpm/<package-manager>.bat`)
- PowerShell scripts (`.fnpm/<package-manager>.ps1`)
- Supports both Command Prompt and PowerShell

## Supported Commands

The following package manager commands are intercepted and redirected:

| Original Command | FNPM Equivalent | Description |
|-----------------|-----------------|-------------|
| `<pm> install` | `fnpm install` | Install dependencies |
| `<pm> add <pkg>` | `fnpm add <pkg>` | Add package |
| `<pm> remove <pkg>` | `fnpm remove <pkg>` | Remove package |
| `<pm> run <script>` | `fnpm run <script>` | Run script |
| `<pm> list` | `fnpm list` | List packages |
| `<pm> update` | `fnpm update` | Update packages |
| `<pm> cache` | `fnpm cache` | Cache operations |
| `<pm> clean` | `fnpm clean` | Clean cache |

*Note: `<pm>` represents your configured package manager (npm, yarn, pnpm, bun, deno)*

## Advanced Usage

### Bypassing Hooks
If you need to use the original package manager directly:
```bash
# Use full path to bypass hooks
$(which pnpm) add some-package

# Or temporarily disable
unset -f pnpm  # Removes the function override
```

### Custom Hook Paths
Hooks respect the PATH environment variable. If you have fnpm installed in a custom location, update the hook scripts accordingly.

### Multiple Projects
Each project can have its own hook configuration. Hooks are only active in directories with `.fnpm/config.json`.

## File Structure

After setup, your project will have:
```
.fnpm/
├── config.json          # FNPM configuration
├── <package-manager>     # Executable hook script (Unix)
├── <package-manager>.bat # Batch file (Windows)
├── <package-manager>.ps1 # PowerShell script (Windows)
├── aliases.sh           # Shell aliases and functions
└── setup.sh             # Activation script
```

## Troubleshooting

### Hooks Not Working
1. Check if hooks are properly activated:
   ```bash
   fnpm hooks status
   ```

2. Ensure you've sourced the setup script:
   ```bash
   source .fnpm/setup.sh
   ```

3. Verify PATH includes `.fnpm` directory:
   ```bash
   echo $PATH | grep .fnpm
   ```

### Permission Issues (Unix)
If hook scripts aren't executable:
```bash
chmod +x .fnpm/<package-manager>
```

### Conflicting Aliases
If you have existing aliases for package managers, they may conflict. Check with:
```bash
type pnpm  # Shows if pnpm is aliased or a function
```

## Security Considerations

- Hook scripts are created with appropriate permissions
- Scripts validate FNPM configuration before execution
- Original package managers remain accessible via full paths
- Hooks only activate in directories with FNPM configuration

## Integration with CI/CD

For CI/CD environments, you may want to skip hooks:
```bash
# In CI scripts
fnpm setup --no-hooks npm
```

Or use fnpm commands directly without hooks:
```bash
# Direct fnpm usage (recommended for CI)
fnpm install
fnpm run build
```

## Examples

### Team Onboarding
```bash
# Developer clones project
git clone <project-repo>
cd <project>

# FNPM is already configured, just activate hooks
source .fnpm/setup.sh

# Now they can use their preferred commands
pnpm install        # Actually runs: fnpm install
pnpm add express    # Actually runs: fnpm add express
```

### Gradual Migration
```bash
# Start without hooks for gradual adoption
fnpm setup --no-hooks yarn

# Team gets used to fnpm commands
fnpm install
fnpm add lodash

# Later, enable hooks for seamless experience
fnpm hooks create
source .fnpm/setup.sh

# Now yarn commands work through fnpm
yarn add express    # Redirected to: fnpm add express
```

## Best Practices

1. **Add setup to shell profile**: For permanent activation
   ```bash
   echo 'source .fnpm/setup.sh' >> ~/.bashrc
   ```

2. **Document for team**: Let team members know about hook activation

3. **CI/CD considerations**: Use `--no-hooks` in automated environments

4. **Version control**: Add `.fnpm/` to `.gitignore` (done automatically)

5. **Testing**: Verify hooks work with `fnpm hooks status`

## Compatibility

- **Shell**: bash, zsh, fish (with bash compatibility)
- **OS**: macOS, Linux, Windows (WSL, Command Prompt, PowerShell)
- **Package Managers**: npm, yarn, pnpm, bun, deno
- **Node.js**: All versions supported by the package managers
