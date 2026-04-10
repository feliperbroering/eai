#!/usr/bin/env bash
set -euo pipefail

# eai installer
# curl -fsSL https://raw.githubusercontent.com/feliperbroering/eai/main/install.sh | bash

REPO="feliperbroering/eai"
INSTALL_DIR="${EAI_INSTALL_DIR:-$HOME/.local/bin}"
BINARY_NAME="eai"

info()  { printf "\033[1;34m▶\033[0m %s\n" "$1"; }
ok()    { printf "\033[1;32m✓\033[0m %s\n" "$1"; }
err()   { printf "\033[1;31m✗\033[0m %s\n" "$1" >&2; exit 1; }

detect_asset_name() {
  local os arch
  case "$(uname -s)" in
    Darwin) os="darwin" ;;
    Linux)  os="linux" ;;
    *)      err "Unsupported OS: $(uname -s)" ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64)  arch="amd64" ;;
    arm64|aarch64) arch="arm64" ;;
    *)             err "Unsupported architecture: $(uname -m)" ;;
  esac
  echo "eai-${os}-${arch}"
}

get_latest_version() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
}

main() {
  info "Installing eai..."

  local asset version url

  asset="$(detect_asset_name)"
  info "Platform: ${asset}"

  version="$(get_latest_version)" || err "Could not fetch latest release."
  info "Version: ${version}"

  url="https://github.com/${REPO}/releases/download/${version}/${asset}"

  info "Downloading ${url}..."
  mkdir -p "$INSTALL_DIR"
  curl -fsSL "$url" -o "${INSTALL_DIR}/${BINARY_NAME}" \
    || err "Download failed. Check: https://github.com/${REPO}/releases"
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

  ok "Installed eai to ${INSTALL_DIR}/${BINARY_NAME}"

  if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo ""
    info "Add this to your shell profile (~/.zshrc or ~/.bashrc):"
    echo ""
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo ""
  fi

  echo ""
  ok "Done! Run: eai setup"
}

main
