#!/usr/bin/env bash
set -e

# Define repository details (User should modify this with their GitHub username)
REPO="aryan/dotengine"
BINARY_NAME="dotengine"

echo -e "\x1b[38;2;129;140;248m"
echo "  ____        __                  _             "
echo " / __ \____  / /____  ____  ____ _(_)___  ___   "
echo "/ / / / __ \/ __/ _ \/ __ \/ __ \`/ / __ \/ _ \  "
echo "/ /_/ / /_/ / /_/  __/ / / / /_/ / / / / /  __/  "
echo "/_____/\____/\__/\___/_/ /_/\__, /_/_/ /_/\___/   "
echo "                           /____/                 "
echo -e "\x1b[0m"
echo -e "\x1b[38;2;166;227;161m✔ Starting Dotengine CLI Installation...\x1b[0m"

# Detect Operating System and Architecture
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
  linux)
    PLATFORM="unknown-linux-gnu"
    ;;
  darwin)
    PLATFORM="apple-darwin"
    ;;
  *)
    echo -e "\x1b[38;2;243;139;168m✖ Error: Unsupported operating system: $OS\x1b[0m"
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64)
    TARGET_ARCH="x86_64"
    ;;
  arm64|aarch64)
    TARGET_ARCH="aarch64"
    ;;
  *)
    echo -e "\x1b[38;2;243;139;168m✖ Error: CPU architecture '$ARCH' is not supported by pre-compiled releases.\x1b[0m"
    exit 1
    ;;
esac

# Fetch the latest release tag from GitHub Releases API
echo -e "\x1b[38;2;137;180;250mℹ Fetching latest release info from GitHub...\x1b[0m"
RELEASE_TAG=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep -Po '"tag_name": "\K[^"]*')

if [ -z "$RELEASE_TAG" ]; then
  echo -e "\x1b[38;2;243;139;168m✖ Error: No stable releases found in GitHub repository '$REPO'.\x1b[0m"
  echo "Please check if your repository is public and has at least one drafted release tag."
  exit 1
fi

ASSET_NAME="${BINARY_NAME}-${TARGET_ARCH}-${PLATFORM}"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/${RELEASE_TAG}/${ASSET_NAME}"

# Define installation target folder
INSTALL_DIR="/usr/local/bin"
USE_SUDO=false

if [ ! -w "$INSTALL_DIR" ]; then
  # If /usr/local/bin is not writable by current user, try ~/.local/bin as a safe fallback
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
  if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    WARN_PATH=true
  fi
else
  # If writable, check if we need sudo just in case, but usually writable means we can copy directly
  USE_SUDO=false
fi

echo -e "\x1b[38;2;137;180;250mℹ Downloading $BINARY_NAME $RELEASE_TAG for $OS ($ARCH)...\x1b[0m"
TEMP_FILE=$(mktemp)
curl -fsSL -o "$TEMP_FILE" "$DOWNLOAD_URL"

echo -e "\x1b[38;2;137;180;250mℹ Applying permissions and moving binary to $INSTALL_DIR...\x1b[0m"
chmod +x "$TEMP_FILE"

if [ "$USE_SUDO" = true ]; then
  sudo mv "$TEMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
else
  mv "$TEMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
fi

echo -e "\x1b[38;2;166;227;161m✔ Successfully installed $BINARY_NAME to $INSTALL_DIR/$BINARY_NAME\x1b[0m"

if [ "$WARN_PATH" = true ]; then
  echo -e "\x1b[38;2;250;179;135m⚠ Warning: '$INSTALL_DIR' is not in your system \$PATH!\x1b[0m"
  echo "Please add it by appending this line to your terminal RC profile (~/.bashrc or ~/.zshrc):"
  echo -e "  \x1b[1mexport PATH=\"\$HOME/.local/bin:\$PATH\"\x1b[0m"
  echo "And then reload your shell: source ~/.bashrc"
fi

echo -e "\x1b[38;2;129;140;248m⚡ Setup complete! Try running: $BINARY_NAME --help\x1b[0m"
