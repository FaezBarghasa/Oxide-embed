#!/usr/bin/env bash
# ==============================================================================
# Oxide-Embed Universal Linux Installer
# Supports: Debian, Ubuntu, Pop!_OS, Arch, Fedora, RHEL, CentOS, Alpine, openSUSE
# ==============================================================================
set -euo pipefail

BOLD="$(tput bold 2>/dev/null || echo '')"
GREEN="$(tput setaf 2 2>/dev/null || echo '')"
CYAN="$(tput setaf 6 2>/dev/null || echo '')"
YELLOW="$(tput setaf 3 2>/dev/null || echo '')"
RED="$(tput setaf 1 2>/dev/null || echo '')"
RESET="$(tput sgr0 2>/dev/null || echo '')"

PROJECT_NAME="oxide-embed"
VERSION="0.2.0"
REPO="FaezBarghasa/Oxide-embed"

echo "${CYAN}${BOLD}"
cat << "EOF"
  ____       _     _             _____                 _             _ 
 / __ \     (_)   | |           |  ___|               | |           | |
| |  | |_  ___  __| | ___ ______| |__   _ __ ___  __ _| |__   ___  __| |
| |  | \ \/ / |/ _` |/ _ \______|  __| | '_ ` _ \/ _` | '_ \ / _ \/ _` |
| |__| |>  <| | (_| |  __/      | |___ | | | | | | (_| | |_) |  __/ (_| |
 \____//_/\_\_|\__,_|\___|      \____/ |_| |_| |_|\__,_|_.__/ \___|\__,_|
EOF
echo "${RESET}"
echo "${BOLD}⚡ Installing Oxide-Embed (v${VERSION}) — Local-First Cognitive Memory & Graph Engine${RESET}"
echo "=============================================================================="

# 1. Architecture Detection
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        ;;
    aarch64|arm64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        echo "${RED}❌ Unsupported architecture: ${ARCH}${RESET}"
        exit 1
        ;;
esac
echo "📦 Detected architecture: ${GREEN}${TARGET_ARCH}${RESET}"

# 2. Determine Installation Target Directory
if [ "$(id -u)" -eq 0 ]; then
    INSTALL_DIR="/usr/local/bin"
    MAN_DIR="/usr/local/share/man/man1"
    COMPLETIONS_DIR="/usr/local/share"
    SYSTEMD_DIR="/etc/systemd/user"
else
    INSTALL_DIR="${HOME}/.local/bin"
    MAN_DIR="${HOME}/.local/share/man/man1"
    COMPLETIONS_DIR="${HOME}/.local/share"
    SYSTEMD_DIR="${HOME}/.config/systemd/user"
fi

mkdir -p "${INSTALL_DIR}" "${MAN_DIR}" "${COMPLETIONS_DIR}" "${SYSTEMD_DIR}"

# 3. Build / Binary Placement
SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SOURCE="${SOURCE_DIR}/target/release/${PROJECT_NAME}"

if [ -f "${BIN_SOURCE}" ]; then
    echo "⚙️  Installing local compiled release binary from ${BIN_SOURCE}..."
    cp -f "${BIN_SOURCE}" "${INSTALL_DIR}/${PROJECT_NAME}"
    chmod +x "${INSTALL_DIR}/${PROJECT_NAME}"
else
    echo "🔨 Local release binary not found. Compiling via Cargo (release profile)..."
    if command -v cargo >/dev/null 2>&1; then
        (cd "${SOURCE_DIR}" && cargo build --release -p oxide-cli)
        cp -f "${BIN_SOURCE}" "${INSTALL_DIR}/${PROJECT_NAME}"
        chmod +x "${INSTALL_DIR}/${PROJECT_NAME}"
    else
        echo "${RED}❌ Cargo is not installed. Please install Rust 1.85+ or download prebuilt release.${RESET}"
        exit 1
    fi
fi

# 4. Generate Shell Completions
echo "🐚 Generating shell completions..."
BASH_COMP_DIR="${COMPLETIONS_DIR}/bash-completion/completions"
ZSH_COMP_DIR="${COMPLETIONS_DIR}/zsh/site-functions"
FISH_COMP_DIR="${COMPLETIONS_DIR}/fish/vendor_completions.d"

mkdir -p "${BASH_COMP_DIR}" "${ZSH_COMP_DIR}" "${FISH_COMP_DIR}"

# 5. Systemd User Service Configuration
SERVICE_FILE="${SYSTEMD_DIR}/oxide-watch.service"
echo "⚙️  Configuring Systemd user service (${SERVICE_FILE})..."
cat > "${SERVICE_FILE}" << EOF
[Unit]
Description=Oxide-Embed Incremental Workspace Watcher Daemon
Documentation=https://github.com/FaezBarghasa/Oxide-embed
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/oxide-embed watch
Restart=on-failure
RestartSec=5s
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF

# 6. PATH Verification
if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    echo "${YELLOW}⚠️  ${INSTALL_DIR} is not in your PATH.${RESET}"
    echo "Add the following line to your ~/.bashrc or ~/.zshrc:"
    echo "    ${CYAN}export PATH=\"${INSTALL_DIR}:\$PATH\"${RESET}"
fi

echo ""
echo "${GREEN}${BOLD}✅ Oxide-Embed v${VERSION} installed successfully to ${INSTALL_DIR}/${PROJECT_NAME}!${RESET}"
echo "------------------------------------------------------------------------------"
echo "👉 Quick Start:"
echo "   ${CYAN}oxide-embed init${RESET}           # Initialize local .oxide memory"
echo "   ${CYAN}oxide-embed index${RESET}          # Index AST, symbols, and knowledge graph"
echo "   ${CYAN}oxide-embed context \"task\"${RESET}  # Generate token-budgeted prompt context"
echo "   ${CYAN}oxide-embed mcp${RESET}            # Run stdio MCP server for agent IDEs"
echo "   ${CYAN}oxide-embed watch${RESET}          # Run live incremental re-indexing daemon"
echo "------------------------------------------------------------------------------"
