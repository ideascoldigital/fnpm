#!/bin/bash

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--version)
            VERSION="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [-v|--version <version>]"
            exit 1
            ;;
    esac
done

# If no version specified, fetch latest
if [ -z "$VERSION" ]; then
    echo "Fetching latest version..."
    VERSION=$(curl -s https://api.github.com/repos/ideascoldigital/fnpm/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        echo "Error: Could not fetch latest version"
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
curl -L "$FNPM_URL" -o "$TARGET_DIR/fnpm"

echo "Making fnpm executable..."
chmod +x "$TARGET_DIR/fnpm"

# Add to PATH if not already there
if [[ ":$PATH:" != *":$TARGET_DIR:"* ]]; then
    echo "Adding $TARGET_DIR to your PATH..."
    # Detect shell configuration file
    if [ -n "$ZSH_VERSION" ]; then
        SHELL_RC="$HOME/.zshrc"
    elif [ -n "$BASH_VERSION" ]; then
        if [ "$OS" = "Darwin" ]; then
            SHELL_RC="$HOME/.bash_profile"
        else
            SHELL_RC="$HOME/.bashrc"
        fi
    else
        SHELL_RC="$HOME/.profile"
    fi

    echo "Adding PATH to $SHELL_RC"
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$SHELL_RC"
    
    # Source the configuration file in the current shell
    if [ -f "$SHELL_RC" ]; then
        export PATH="$HOME/.local/bin:$PATH"
        echo "PATH has been updated in current session"
    fi
fi

# Verify installation
echo "Verifying installation..."

# Try to execute fnpm directly
if "$TARGET_DIR/fnpm" --version >/dev/null 2>&1; then
    echo "✅ fnpm has been successfully installed and is accessible"
    echo "You can now use 'fnpm' from anywhere!"
else
    echo "⚠️  Installation might have failed. Please check the following:"
    echo "1. Verify that $TARGET_DIR/fnpm exists and is executable"
    echo "2. Ensure your PATH includes $TARGET_DIR"
    echo "3. Try running: source $SHELL_RC"
    exit 1
fi
