#!/bin/bash

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--version)
            VERSION="$2"
            shift 2
            ;;
        remove)
            echo "Removing fnpm binary..."
            if [ -f "$HOME/.local/bin/fnpm" ]; then
                rm "$HOME/.local/bin/fnpm"
                echo "✅ fnpm binary has been removed successfully"
            else
                echo "⚠️  fnpm binary not found in $HOME/.local/bin"
            fi
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [-v|--version <version>] [remove]"
            exit 1
            ;;
    esac
done

# If no version specified, fetch latest
if [ -z "$VERSION" ]; then
    echo "Fetching latest version..."
    VERSION_RESPONSE=$(curl -sL --connect-timeout 10 https://api.github.com/repos/ideascoldigital/fnpm/releases/latest)
    if [ $? -ne 0 ]; then
        echo "❌ Error: Failed to connect to GitHub API. Please check your internet connection."
        exit 1
    fi
    VERSION=$(echo "$VERSION_RESPONSE" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        echo "❌ Error: Could not fetch latest version. API response was unexpected."
        echo "API Response: $VERSION_RESPONSE"
        exit 1
    fi
    echo "Latest version is: $VERSION"
else
    echo "Installing specific version: $VERSION"
fi

TARGET_DIR="$HOME/.local/bin"

# Detect OS and architecture
OS=$(uname -s)
ARCH=$(uname -m)

# Determine which binary to download
if [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "arm64" ]; then
        BINARY="fnpm-macos-arm64"
    elif [ "$ARCH" = "x86_64" ]; then
        BINARY="fnpm-macos-amd64"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
elif [ "$OS" = "Linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        BINARY="fnpm-linux-amd64"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
else
    echo "Unsupported operating system: $OS"
    exit 1
fi

FNPM_URL="https://github.com/ideascoldigital/fnpm/releases/download/$VERSION/$BINARY"

# Create target directory if it doesn't exist
mkdir -p "$TARGET_DIR"

echo "Detected OS: $OS"
echo "Detected Architecture: $ARCH"
echo "Downloading $BINARY..."
if ! curl -L --progress-bar --connect-timeout 10 "$FNPM_URL" -o "$TARGET_DIR/fnpm"; then
    echo "❌ Error: Failed to download fnpm binary. Please check your internet connection."
    exit 1
fi

echo "Making fnpm executable..."
chmod +x "$TARGET_DIR/fnpm"

# Add to PATH if not already there
if [[ ":$PATH:" != *":$TARGET_DIR:"* ]]; then
    echo "Adding $TARGET_DIR to your PATH..."
    # Detect current shell and set appropriate config file
    # First try to detect shell from environment variables
    if [ -n "$ZSH_VERSION" ]; then
        CURRENT_SHELL="zsh"
    elif [ -n "$BASH_VERSION" ]; then
        CURRENT_SHELL="bash"
    else
        # Enhanced shell detection logic
        # Check if running in zsh
        if [ -n "$ZSH_NAME" ] || [ "$SHELL" = "/bin/zsh" ] || [ "$SHELL" = "/usr/bin/zsh" ] || [[ "$SHELL" =~ zsh$ ]]; then
            CURRENT_SHELL="zsh"
        else
            # Try to get shell from SHELL environment variable
            CURRENT_SHELL=$(basename "$SHELL" 2>/dev/null || echo "")
            if [ -z "$CURRENT_SHELL" ]; then
                # If SHELL is empty, try to get from process info
                CURRENT_SHELL=$(ps -p $$ -o comm= 2>/dev/null || echo "")
                # Remove any leading dash if present (login shell)
                CURRENT_SHELL=${CURRENT_SHELL#-}
                # Additional check for zsh in process name
                if [[ "$CURRENT_SHELL" =~ zsh ]]; then
                    CURRENT_SHELL="zsh"
                fi
            fi
        fi
    fi
    echo "Detected shell: $CURRENT_SHELL"
    if [ -n "$ZSH_VERSION" ] || [ "$CURRENT_SHELL" = "zsh" ]; then
        echo "Configuring for ZSH..."
        SHELL_RC="$HOME/.zshrc"
        # Create .zshrc if it doesn't exist
        if [ ! -f "$SHELL_RC" ]; then
            echo "Creating $SHELL_RC as it doesn't exist"
            touch "$SHELL_RC"
        fi
        
        # Add PATH to .zshenv for immediate effect
        SHELL_ENV="$HOME/.zshenv"
        if [ ! -f "$SHELL_ENV" ]; then
            echo "Creating $SHELL_ENV as it doesn't exist"
            touch "$SHELL_ENV"
        fi
        echo "Adding PATH to $SHELL_ENV"
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$SHELL_ENV"
    elif [ "$CURRENT_SHELL" = "bash" ]; then
        echo "Configuring for Bash..."
        if [ "$OS" = "Darwin" ]; then
            echo "Using .bash_profile for macOS"
            SHELL_RC="$HOME/.bash_profile"
        else
            echo "Using .bashrc for Linux"
            SHELL_RC="$HOME/.bashrc"
        fi
    else
        echo "Using default .profile configuration"
        SHELL_RC="$HOME/.profile"
    fi

    echo "Adding PATH to $SHELL_RC"
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$SHELL_RC"
    echo "PATH configuration added to $SHELL_RC"
    
    # Source the configuration files in the current shell
    if [ "$CURRENT_SHELL" = "zsh" ]; then
        echo "Updating current shell environment for ZSH..."
        export PATH="$HOME/.local/bin:$PATH"
        echo "Sourcing ZSH configuration files..."
        source "$SHELL_ENV" 2>/dev/null && echo "Successfully sourced $SHELL_ENV" || echo "Note: Could not source $SHELL_ENV"
        source "$SHELL_RC" 2>/dev/null && echo "Successfully sourced $SHELL_RC" || echo "Note: Could not source $SHELL_RC"
        echo "PATH has been updated in current session"
        echo "✨ ZSH configuration has been updated"
    elif [ -f "$SHELL_RC" ]; then
        export PATH="$HOME/.local/bin:$PATH"
        source "$SHELL_RC" 2>/dev/null || true
        echo "PATH has been updated in current session"
    fi
fi

# Verify installation
echo "Verifying installation..."

# Try to execute fnpm directly
if "$TARGET_DIR/fnpm" --version >/dev/null 2>&1; then
    echo "✅ fnpm has been successfully installed and is accessible"
    echo "You can now use 'fnpm' from anywhere!"
    echo ""
    echo "Shell Configuration Summary:"
    echo "------------------------"
    echo "Current Shell: $CURRENT_SHELL"
    echo "Shell Environment: $SHELL"
    echo "Target Directory: $TARGET_DIR"
    echo ""
    echo "🔥 If 'fnpm' command is not found, please run one of these commands based on your shell:"

    if [ -n "$ZSH_VERSION" ] || [ "$CURRENT_SHELL" = "zsh" ]; then
        echo "For Zsh users:"
        echo "    source ~/.zshenv    # for immediate effect"
        echo "    source ~/.zshrc     # for persistent configuration"
    elif [ "$CURRENT_SHELL" = "bash" ]; then
        if [ "$OS" = "Darwin" ]; then
            echo "For Bash users on macOS:"
            echo "    source ~/.bash_profile"
        else
            echo "For Bash users on Linux:"
            echo "    source ~/.bashrc"
        fi
    else
        echo "For other shells:"
        echo "    source ~/.profile"
    fi
else
    echo "⚠️  Installation might have failed. Please check the following:"
    echo "1. Verify that $TARGET_DIR/fnpm exists and is executable"
    echo "2. Ensure your PATH includes $TARGET_DIR"
    echo "3. Try running: source $SHELL_RC"
    exit 1
fi
