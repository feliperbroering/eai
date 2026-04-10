#!/usr/bin/env bash
set -euo pipefail

# eai installer
# curl -fsSL https://raw.githubusercontent.com/feliperbroering/eai/main/install.sh | bash

REPO="feliperbroering/eai"
INSTALL_DIR="${EAI_INSTALL_DIR:-$HOME/.local/bin}"
BINARY_NAME="eai"
OLLAMA_MODEL="qwen3:4b"

# --- helpers ---

info()  { printf "\033[1;34m▶\033[0m %s\n" "$1"; }
ok()    { printf "\033[1;32m✓\033[0m %s\n" "$1"; }
err()   { printf "\033[1;31m✗\033[0m %s\n" "$1" >&2; exit 1; }

ollama_ready() {
  curl -fsS "http://127.0.0.1:11434/api/tags" >/dev/null 2>&1
}

ensure_ollama_running() {
  if ollama_ready; then
    return 0
  fi

  info "Starting Ollama..."
  nohup ollama serve >/tmp/eai-ollama.log 2>&1 &

  for _ in $(seq 1 20); do
    if ollama_ready; then
      return 0
    fi
    sleep 1
  done

  err "Ollama did not become ready at http://127.0.0.1:11434"
}

ensure_ollama_and_model() {
  if ! command -v ollama &>/dev/null; then
    info "Ollama not found. Installing via the official installer..."
    OLLAMA_NO_START=1 curl -fsSL "https://ollama.com/install.sh" | sh \
      || err "Ollama installation failed"
  fi

  command -v ollama &>/dev/null || err "Ollama installation finished but the binary is not in PATH"

  ensure_ollama_running

  if ollama list 2>/dev/null | awk 'NR > 1 {print $1}' | grep -qx "$OLLAMA_MODEL"; then
    ok "Ollama model ${OLLAMA_MODEL} detected."
    return 0
  fi

  info "Pulling Ollama model ${OLLAMA_MODEL}..."
  ollama pull "$OLLAMA_MODEL" || err "Failed to pull ${OLLAMA_MODEL}"
}

detect_platform() {
  local os arch

  case "$(uname -s)" in
    Darwin) os="apple-darwin" ;;
    Linux)  os="unknown-linux-gnu" ;;
    *)      err "Unsupported OS: $(uname -s)" ;;
  esac

  case "$(uname -m)" in
    x86_64|amd64)  arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *)             err "Unsupported architecture: $(uname -m)" ;;
  esac

  echo "${arch}-${os}"
}

get_latest_version() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
}

# --- main ---

main() {
  info "Installing eai..."

  local platform version archive_name url tmp_dir

  platform="$(detect_platform)"
  info "Platform: ${platform}"

  version="$(get_latest_version)" || err "Could not fetch latest release. Is there a release published?"
  info "Version: ${version}"

  archive_name="eai-${version}-${platform}.tar.gz"
  url="https://github.com/${REPO}/releases/download/${version}/${archive_name}"

  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' EXIT

  info "Downloading ${url}..."
  curl -fsSL "$url" -o "${tmp_dir}/${archive_name}" \
    || err "Download failed. Check if the release exists at: https://github.com/${REPO}/releases"

  info "Extracting..."
  tar -xzf "${tmp_dir}/${archive_name}" -C "$tmp_dir"

  mkdir -p "$INSTALL_DIR"
  mv "${tmp_dir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

  ok "Installed eai to ${INSTALL_DIR}/${BINARY_NAME}"

  # Check if INSTALL_DIR is in PATH
  if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo ""
    info "Add this to your shell profile (~/.zshrc or ~/.bashrc):"
    echo ""
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo ""
  fi

  ensure_ollama_and_model
  ok "Ollama is ready with ${OLLAMA_MODEL}."

  echo ""
  ok "Done! Try: eai \"list all files modified today\""
}

main
