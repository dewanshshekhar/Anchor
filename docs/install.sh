#!/usr/bin/env bash
# Anchor installer (production-grade)
# Usage:
#   curl -fsSL https://tharun-10dragneel.github.io/Anchor/install.sh | bash
#

set -euo pipefail

REPO="Tharun-10Dragneel/Anchor"

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

mkdir -p "$INSTALL_DIR"

echo "Installing Anchor → $INSTALL_DIR"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  darwin)
    case "$ARCH" in
      x86_64) BINARY="anchor-macos-intel" ;;
      arm64)  BINARY="anchor-macos-arm" ;;
      *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  linux)
    case "$ARCH" in
      x86_64) BINARY="anchor-linux-x64" ;;
      *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases" \
  | grep '"tag_name"' | head -1 | cut -d'"' -f4)

[ -z "$LATEST" ] && { echo "Failed to get latest release"; exit 1; }

echo "Version: $LATEST"

URL="https://github.com/$REPO/releases/download/$LATEST/$BINARY.tar.gz"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

curl -fsSL "$URL" | tar -xz -C "$TMP"

FILES=("anchor" "anchor-mcp")

install_file () {
  src="$1"
  dest="$2"

  if [ -w "$INSTALL_DIR" ]; then
    mv "$src" "$dest"
  else
    echo "Requesting sudo permission..."
    sudo mv "$src" "$dest"
  fi
}

for f in "${FILES[@]}"; do
  install_file "$TMP/$f" "$INSTALL_DIR/$f"
  chmod +x "$INSTALL_DIR/$f"
done

echo ""
echo " █████╗ ███╗   ██╗ ██████╗██╗  ██╗ ██████╗ ██████╗"
echo "██╔══██╗████╗  ██║██╔════╝██║  ██║██╔═══██╗██╔══██╗"
echo "███████║██╔██╗ ██║██║     ███████║██║   ██║██████╔╝"
echo "██╔══██║██║╚██╗██║██║     ██╔══██║██║   ██║██╔══██╗"
echo "██║  ██║██║ ╚████║╚██████╗██║  ██║╚██████╔╝██║  ██║"
echo "╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝"
echo "       Code Intelligence for AI Agents"
echo ""
echo "Get started:"
echo "  cd your-project"
echo "  anchor build"
echo "  anchor map"
echo ""
echo "Update:    anchor update"
echo "Uninstall: curl -fsSL https://tharun-10dragneel.github.io/Anchor/uninstall.sh | bash"
echo ""

# PATH hint
if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
  echo "Tip: add ~/.local/bin to PATH:"
  echo '  export PATH="$HOME/.local/bin:$PATH"'
fi
