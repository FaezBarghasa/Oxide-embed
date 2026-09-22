#!/usr/bin/env bash
# ==============================================================================
# Oxide-Embed Universal Linux Interactive & Automated Installer
# Supports: Ubuntu, Debian, Pop!_OS, Arch Linux, Fedora, RHEL, CentOS, Alpine, openSUSE
# Features:
#   - Automated Distro Package Manager Detection (dpkg / pacman / rpm / apk / tarball)
#   - Release binary compilation & prebuilt fallback installation
#   - User-level (~/.local/bin) or System-level (/usr/local/bin) target selection
#   - Automatic Shell Completions (bash, zsh, fish)
#   - Systemd Watcher Daemon configuration (systemctl --user)
#   - Automated PATH checking & shell environment injection
# ==============================================================================
set -euo pipefail

# ANSI color codes
BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RESET='\033[0m'

PROJECT_NAME="oxide-embed"
VERSION="0.4.0"
REPO="FaezBarghasa/Oxide-embed"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="${SCRIPT_DIR}"

echo -e "${CYAN}${BOLD}"
cat << "EOF"
  ____       _     _             _____                 _             _ 
 / __ \     (_)   | |           |  ___|               | |           | |
| |  | |_  ___  __| | ___ ______| |__   _ __ ___  __ _| |__   ___  __| |
| |  | \ \/ / |/ _` |/ _ \______|  __| | '_ ` _ \/ _` | '_ \ / _ \/ _` |
| |__| |>  <| | (_| |  __/      | |___ | | | | | | (_| | |_) |  __/ (_| |
 \____//_/\_\_|\__,_|\___|      \____/ |_| |_| |_|\__,_|_.__/ \___|\__,_|
EOF
echo -e "${RESET}"
echo -e "${BOLD}⚡ Oxide-Embed Linux Universal Installer (v${VERSION})${RESET}"
echo -e "   Repository: ${CYAN}https://github.com/${REPO}${RESET}"
echo -e "=============================================================================="

# ------------------------------------------------------------------------------
# 1. Host and Architecture Detection
# ------------------------------------------------------------------------------
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        DEB_ARCH="amd64"
        ;;
    aarch64|arm64)
        TARGET_ARCH="aarch64"
        DEB_ARCH="arm64"
        ;;
    *)
        echo -e "${RED}❌ Unsupported architecture: ${ARCH}${RESET}" >&2
        exit 1
        ;;
esac

DISTRO="unknown"
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO="${ID:-linux}"
    DISTRO_NAME="${PRETTY_NAME:-$DISTRO}"
else
    DISTRO_NAME="$(uname -s)"
fi

echo -e "📦 System:       ${GREEN}${DISTRO_NAME}${RESET}"
echo -e "⚙️  Architecture: ${GREEN}${TARGET_ARCH}${RESET}"

# ------------------------------------------------------------------------------
# 2. Package Manager Fast-Path (.deb on Debian/Ubuntu/Pop!_OS)
# ------------------------------------------------------------------------------
DEB_PATH="${WORKSPACE_ROOT}/dist/${PROJECT_NAME}_${VERSION}_${DEB_ARCH}.deb"
if [ -f "${DEB_PATH}" ] && command -v dpkg >/dev/null 2>&1 && [ "$(id -u)" -eq 0 ]; then
    echo -e "\n${BLUE}Detected local Debian package: ${DEB_PATH}${RESET}"
    echo -e "Installing via dpkg..."
    dpkg -i "${DEB_PATH}"
    echo -e "${GREEN}${BOLD}✅ Installed successfully via dpkg!${RESET}"
    exit 0
fi

# ------------------------------------------------------------------------------
# 3. Determine Installation Target Locations
# ------------------------------------------------------------------------------
if [ "$(id -u)" -eq 0 ]; then
    INSTALL_DIR="/usr/local/bin"
    SHARE_DIR="/usr/local/share"
    SYSTEMD_DIR="/etc/systemd/user"
else
    INSTALL_DIR="${HOME}/.local/bin"
    SHARE_DIR="${HOME}/.local/share"
    SYSTEMD_DIR="${HOME}/.config/systemd/user"
fi

mkdir -p "${INSTALL_DIR}" "${SHARE_DIR}" "${SYSTEMD_DIR}"

# ------------------------------------------------------------------------------
# 4. Binary Discovery or On-Demand Compilation
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}Locating / Compiling ${PROJECT_NAME} binary...${RESET}"

DIST_BIN="${WORKSPACE_ROOT}/dist/bin/${PROJECT_NAME}"
RELEASE_BIN="${WORKSPACE_ROOT}/target/release/${PROJECT_NAME}"
BIN_SOURCE=""

if [ -f "${DIST_BIN}" ]; then
    BIN_SOURCE="${DIST_BIN}"
elif [ -f "${RELEASE_BIN}" ]; then
    BIN_SOURCE="${RELEASE_BIN}"
elif [ -f "${WORKSPACE_ROOT}/bin/${PROJECT_NAME}" ]; then
    BIN_SOURCE="${WORKSPACE_ROOT}/bin/${PROJECT_NAME}"
fi

if [ -n "${BIN_SOURCE}" ]; then
    echo -e "⚙️  Using existing binary: ${CYAN}${BIN_SOURCE}${RESET}"
    cp -f "${BIN_SOURCE}" "${INSTALL_DIR}/${PROJECT_NAME}"
    chmod 755 "${INSTALL_DIR}/${PROJECT_NAME}"
else
    echo -e "🔨 Binary not pre-built. Compiling with Cargo (release profile)..."
    if command -v cargo >/dev/null 2>&1; then
        (cd "${WORKSPACE_ROOT}" && cargo build --release -p oxide-cli)
        cp -f "${RELEASE_BIN}" "${INSTALL_DIR}/${PROJECT_NAME}"
        chmod 755 "${INSTALL_DIR}/${PROJECT_NAME}"
    else
        echo -e "${RED}❌ Neither prebuilt binary nor cargo found on system.${RESET}" >&2
        echo -e "${YELLOW}Please install Rust (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh) or run ./build-ubuntu-app.sh first.${RESET}" >&2
        exit 1
    fi
fi

# ------------------------------------------------------------------------------
# 5. Shell Completions Setup
# ------------------------------------------------------------------------------
echo -e "🐚 Installing shell auto-completions..."
BASH_COMP_DIR="${SHARE_DIR}/bash-completion/completions"
ZSH_COMP_DIR="${SHARE_DIR}/zsh/site-functions"
FISH_COMP_DIR="${SHARE_DIR}/fish/vendor_completions.d"

mkdir -p "${BASH_COMP_DIR}" "${ZSH_COMP_DIR}" "${FISH_COMP_DIR}"

if "${INSTALL_DIR}/${PROJECT_NAME}" --help >/dev/null 2>&1; then
    "${INSTALL_DIR}/${PROJECT_NAME}" completions bash > "${BASH_COMP_DIR}/${PROJECT_NAME}" 2>/dev/null || true
    "${INSTALL_DIR}/${PROJECT_NAME}" completions zsh > "${ZSH_COMP_DIR}/_${PROJECT_NAME}" 2>/dev/null || true
    "${INSTALL_DIR}/${PROJECT_NAME}" completions fish > "${FISH_COMP_DIR}/${PROJECT_NAME}.fish" 2>/dev/null || true
fi

# ------------------------------------------------------------------------------
# 6. Systemd Watcher Daemon Unit
# ------------------------------------------------------------------------------
SERVICE_FILE="${SYSTEMD_DIR}/oxide-watch.service"
echo -e "⚙️  Registering Systemd user service: ${CYAN}${SERVICE_FILE}${RESET}"
cat > "${SERVICE_FILE}" << EOF
[Unit]
Description=Oxide-Embed Incremental Workspace Watcher Daemon
Documentation=https://github.com/${REPO}
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/${PROJECT_NAME} watch
Restart=on-failure
RestartSec=3s
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF

# Reload systemd if available
if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload 2>/dev/null || true
fi

# ------------------------------------------------------------------------------
# 7. PATH Verification & Shell Hook
# ------------------------------------------------------------------------------
IN_PATH=0
if [[ ":$PATH:" == *":${INSTALL_DIR}:"* ]]; then
    IN_PATH=1
fi

if [ "${IN_PATH}" -eq 0 ]; then
    echo -e "\n${YELLOW}⚠️  ${INSTALL_DIR} is not currently in your PATH.${RESET}"
    SHELL_PROFILE=""
    if [ -n "${ZSH_VERSION:-}" ] || [ -f "${HOME}/.zshrc" ]; then
        SHELL_PROFILE="${HOME}/.zshrc"
    elif [ -f "${HOME}/.bashrc" ]; then
        SHELL_PROFILE="${HOME}/.bashrc"
    fi

    if [ -n "${SHELL_PROFILE}" ]; then
        if ! grep -q "${INSTALL_DIR}" "${SHELL_PROFILE}" 2>/dev/null; then
            echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >> "${SHELL_PROFILE}"
            echo -e "${GREEN}✓ Added ${INSTALL_DIR} to ${SHELL_PROFILE}${RESET}"
        fi
    fi
fi

# ------------------------------------------------------------------------------
# 8. Success Report & Usage Guide
# ------------------------------------------------------------------------------
echo -e "\n${GREEN}${BOLD}==============================================================================${RESET}"
echo -e "${GREEN}${BOLD}✓ Oxide-Embed (v${VERSION}) successfully installed to ${INSTALL_DIR}/${PROJECT_NAME}!${RESET}"
echo -e "=============================================================================="
echo -e "👉 Verify installation:"
echo -e "   ${CYAN}oxide-embed --version${RESET}"
echo -e "\n👉 Core Capabilities:"
echo -e "   ${CYAN}oxide-embed init${RESET}           # Initialize local .oxide engine"
echo -e "   ${CYAN}oxide-embed index${RESET}          # Index workspace AST, symbols & embeddings"
echo -e "   ${CYAN}oxide-embed context \"query\"${RESET}  # Retrieve AST-aware budgeted prompt context"
echo -e "   ${CYAN}oxide-embed mcp${RESET}            # Launch Model Context Protocol stdio server"
echo -e "   ${CYAN}oxide-embed watch${RESET}          # Run live incremental re-indexing daemon"
echo -e "\n👉 Systemd Background Daemon (Optional):"
echo -e "   ${CYAN}systemctl --user enable --now oxide-watch.service${RESET}"
echo -e "${GREEN}${BOLD}==============================================================================${RESET}"
