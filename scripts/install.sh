#!/bin/bash
set -e

# CAWA - Context-Aware Workspace Automation Installer

GITHUB_REPO="mmiraly/cawa"
BINARY_NAME="cs"
INSTALL_DIR="${CS_INSTALL_DIR:-/usr/local/bin}"

echo "✨ CAWA Installer"

# 1. detect os - linux or mac?
OS="$(uname -s)"
case "${OS}" in
    Linux*)     OS_TYPE=linux;;
    Darwin*)    OS_TYPE=darwin;;
    *)          echo "❌ Unsupported OS: ${OS}"; exit 1;;
esac

# 2. check arch - intel or apple silicon
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64)    ARCH_TYPE=amd64;;
    aarch64)   ARCH_TYPE=arm64;;
    arm64)     ARCH_TYPE=arm64;;
    *)         echo "❌ Unsupported Architecture: ${ARCH}"; exit 1;;
esac

echo "🔎 Detected: ${OS_TYPE} / ${ARCH_TYPE}"

# 3. fetch latest tag - hit github api
echo "🌐 Fetching latest version..."
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/${GITHUB_REPO}/releases/latest")
TAG_NAME=$(echo "$LATEST_RELEASE" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$TAG_NAME" ]; then
    echo "❌ Error: Could not find latest release."
    exit 1
fi

echo "⬇️  Downloading version: ${TAG_NAME}"

# 4. construct filename - must match release script format
# ex: cs-v1.0.0-linux-amd64.tar.gz
ASSET_NAME="${BINARY_NAME}-${TAG_NAME}-${OS_TYPE}-${ARCH_TYPE}.tar.gz"
DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${TAG_NAME}/${ASSET_NAME}"

TEMP_DIR=$(mktemp -d)
TAR_FILE="${TEMP_DIR}/${ASSET_NAME}"

# 5. download it - fail fast if missing
echo "URl: $DOWNLOAD_URL"
curl -L -o "$TAR_FILE" "$DOWNLOAD_URL" --fail || {
    echo "❌ Error: Download failed. Could not find asset: ${ASSET_NAME}"
    echo "   Ensure a release exists for your OS/Arch."
    exit 1
}

# 6. unpack - extract to temp
echo "📦 Extracting..."
tar -xzf "$TAR_FILE" -C "$TEMP_DIR"

# 7. install - move to bin dir
echo "hammer: Installing to ${INSTALL_DIR}..."

# try direct copy - else fallback to sudo
if [ -w "$INSTALL_DIR" ]; then
    cp "$TEMP_DIR/$BINARY_NAME" "$INSTALL_DIR/"
else
    echo "🔑 Sudo required to move binary."
    sudo cp "$TEMP_DIR/$BINARY_NAME" "$INSTALL_DIR/"
fi

# 8. cleanup - don't leave trash
rm -rf "$TEMP_DIR"

echo "🎉 Installed '${BINARY_NAME}' successfully to ${INSTALL_DIR}!"
echo "   Try it: cs --help"
