#!/bin/bash

VERSION="v0.0.4"
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
    echo "Please restart your terminal or run 'source $SHELL_RC' to use fnpm from anywhere"
fi

echo "Installation complete! fnpm has been installed to $TARGET_DIR/fnpm"
