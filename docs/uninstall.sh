#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

remove() {
  if [ -e "$1" ]; then
    if [ -w "$INSTALL_DIR" ]; then
      rm -f "$1"
    else
      sudo rm -f "$1"
    fi
  fi
}

remove "$INSTALL_DIR/anchor"
remove "$INSTALL_DIR/anchor-mcp"
