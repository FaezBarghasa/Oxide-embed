#!/usr/bin/env bash
# ==============================================================================
# Oxide-Embed Ubuntu / Debian Full Application Build & Packaging Script
# Generates release binary, .deb package, portable tarball, systemd service,
# shell completions, and standalone distribution bundle in dist/
# ==============================================================================
set -euo pipefail

# ANSI color codes
BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

PROJECT_NAME="oxide-embed"
VERSION="0.4.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="${SCRIPT_DIR}"
DIST_DIR="${WORKSPACE_ROOT}/dist"
BUILD_DIR="${WORKSPACE_ROOT}/target/deb-build"

# Architecture detection
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        DEB_ARCH="amd64"
        TARGET_TRIPLE="x86_64-unknown-linux-gnu"
        ;;
    aarch64|arm64)
        TARGET_ARCH="aarch64"
        DEB_ARCH="arm64"
        TARGET_TRIPLE="aarch64-unknown-linux-gnu"
        ;;
    *)
        TARGET_ARCH="${ARCH}"
        DEB_ARCH="${ARCH}"
        TARGET_TRIPLE="${ARCH}-unknown-linux-gnu"
        ;;
esac

PKG_BASENAME="${PROJECT_NAME}-v${VERSION}-${TARGET_TRIPLE}"
DEB_PKG_NAME="${PROJECT_NAME}_${VERSION}_${DEB_ARCH}.deb"
TARBALL_NAME="${PKG_BASENAME}.tar.gz"

echo -e "${BLUE}${BOLD}"
cat << "EOF"
  ____       _     _             _____                 _             _ 
 / __ \     (_)   | |           |  ___|               | |           | |
| |  | |_  ___  __| | ___ ______| |__   _ __ ___  __ _| |__   ___  __| |
| |  | \ \/ / |/ _` |/ _ \______|  __| | '_ ` _ \/ _` | '_ \ / _ \/ _` |
| |__| |>  <| | (_| |  __/      | |___ | | | | | | (_| | |_) |  __/ (_| |
 \____//_/\_\_|\__,_|\___|      \____/ |_| |_| |_|\__,_|_.__/ \___|\__,_|
EOF
echo -e "${NC}"
echo -e "${CYAN}${BOLD}⚡ Oxide-Embed Ubuntu / Debian Full Application Build (v${VERSION})${NC}"
echo -e "   Architecture: ${GREEN}${TARGET_ARCH} (${DEB_ARCH})${NC} | Host: $(lsb_release -ds 2>/dev/null || cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2 | tr -d '\"' || uname -s)"
echo -e "=============================================================================="

# ------------------------------------------------------------------------------
# 1. Dependency Validation
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}[1/6] Validating build toolchain and system dependencies...${NC}"

REQUIRED_BINS=("cargo" "rustc" "dpkg-deb" "tar" "gzip" "sha256sum" "strip")
MISSING_BINS=()
for b in "${REQUIRED_BINS[@]}"; do
    if ! command -v "$b" >/dev/null 2>&1; then
        MISSING_BINS+=("$b")
    fi
done

if [ ${#MISSING_BINS[@]} -gt 0 ]; then
    echo -e "${RED}❌ Missing required tools: ${MISSING_BINS[*]}${NC}" >&2
    echo -e "${YELLOW}Please install missing tools (e.g.: sudo apt-get install -y build-essential dpkg-dev)${NC}" >&2
    exit 1
fi
echo -e "${GREEN}✓ Toolchain verified (Rust $(rustc --version | awk '{print $2}'))${NC}"

# ------------------------------------------------------------------------------
# 2. Build Release Binary with Cargo
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}[2/6] Compiling optimized release binary (oxide-cli)...${NC}"
mkdir -p "${DIST_DIR}"
cd "${WORKSPACE_ROOT}"
cargo build --release -p oxide-cli

RELEASE_BIN="${WORKSPACE_ROOT}/target/release/${PROJECT_NAME}"
if [ ! -f "${RELEASE_BIN}" ]; then
    echo -e "${RED}❌ Release binary not found at ${RELEASE_BIN}${NC}" >&2
    exit 1
fi

BIN_SIZE_RAW="$(stat -c%s "${RELEASE_BIN}" 2>/dev/null || stat -f%z "${RELEASE_BIN}" 2>/dev/null || echo 0)"
BIN_SIZE_MB="$(awk "BEGIN {printf \"%.2f\", ${BIN_SIZE_RAW}/1048576}")"
echo -e "${GREEN}✓ Compiled release binary: ${RELEASE_BIN} (${BIN_SIZE_MB} MB)${NC}"

# ------------------------------------------------------------------------------
# 3. Assemble Debian (.deb) Package
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}[3/6] Building Debian/Ubuntu package (.deb)...${NC}"
DEB_STAGE="${BUILD_DIR}/${PROJECT_NAME}_${VERSION}_${DEB_ARCH}"
rm -rf "${BUILD_DIR}"

mkdir -p "${DEB_STAGE}/DEBIAN"
mkdir -p "${DEB_STAGE}/usr/bin"
mkdir -p "${DEB_STAGE}/usr/lib/systemd/user"
mkdir -p "${DEB_STAGE}/usr/share/doc/${PROJECT_NAME}"
mkdir -p "${DEB_STAGE}/usr/share/bash-completion/completions"
mkdir -p "${DEB_STAGE}/usr/share/zsh/site-functions"
mkdir -p "${DEB_STAGE}/usr/share/fish/vendor_completions.d"

# Binary
cp "${RELEASE_BIN}" "${DEB_STAGE}/usr/bin/${PROJECT_NAME}"
strip "${DEB_STAGE}/usr/bin/${PROJECT_NAME}"
chmod 755 "${DEB_STAGE}/usr/bin/${PROJECT_NAME}"

# Control and lifecycle scripts
CONTROL_FILE="${DEB_STAGE}/DEBIAN/control"
if [ -f "${WORKSPACE_ROOT}/packaging/debian/control" ]; then
    cp "${WORKSPACE_ROOT}/packaging/debian/control" "${CONTROL_FILE}"
else
    cat > "${CONTROL_FILE}" << EOF
Package: ${PROJECT_NAME}
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${DEB_ARCH}
Maintainer: Faez Barghasa <faez.barghasa@gmail.com>
Depends: libc6 (>= 2.34), libgcc-s1 (>= 4.2)
Description: Local-first code graph indexing, cognitive memory, and context hygiene engine
 Oxide-Embed is an ultra-fast, local-first code graph and memory engine engineered
 in pure Rust. It combines Tree-sitter AST extraction, Hugging Face Candle
 embeddings, SurrealDB 3.x embedded graph storage, terminal error condensing,
 session handover state tracking, and Model Context Protocol (MCP) server capabilities.
 Homepage: https://github.com/FaezBarghasa/Oxide-embed
EOF
fi

# Sync dynamic fields
sed -i "s/^Version:.*/Version: ${VERSION}/" "${CONTROL_FILE}"
sed -i "s/^Architecture:.*/Architecture: ${DEB_ARCH}/" "${CONTROL_FILE}"

if [ -f "${WORKSPACE_ROOT}/packaging/debian/postinst" ]; then
    cp "${WORKSPACE_ROOT}/packaging/debian/postinst" "${DEB_STAGE}/DEBIAN/postinst"
    chmod 755 "${DEB_STAGE}/DEBIAN/postinst"
fi

if [ -f "${WORKSPACE_ROOT}/packaging/debian/prerm" ]; then
    cp "${WORKSPACE_ROOT}/packaging/debian/prerm" "${DEB_STAGE}/DEBIAN/prerm"
    chmod 755 "${DEB_STAGE}/DEBIAN/prerm"
fi

# Docs and Systemd service
cp "${WORKSPACE_ROOT}/README.md" "${DEB_STAGE}/usr/share/doc/${PROJECT_NAME}/README.md"
cp "${WORKSPACE_ROOT}/CHANGELOG.md" "${DEB_STAGE}/usr/share/doc/${PROJECT_NAME}/CHANGELOG.md" 2>/dev/null || true
if [ -f "${WORKSPACE_ROOT}/packaging/debian/copyright" ]; then
    cp "${WORKSPACE_ROOT}/packaging/debian/copyright" "${DEB_STAGE}/usr/share/doc/${PROJECT_NAME}/copyright"
fi
if [ -f "${WORKSPACE_ROOT}/packaging/systemd/oxide-watch.service" ]; then
    cp "${WORKSPACE_ROOT}/packaging/systemd/oxide-watch.service" "${DEB_STAGE}/usr/lib/systemd/user/oxide-watch.service"
fi

# Shell completions (if binary supports --generate-completions / clap)
"${DEB_STAGE}/usr/bin/${PROJECT_NAME}" completions bash > "${DEB_STAGE}/usr/share/bash-completion/completions/${PROJECT_NAME}" 2>/dev/null || true
"${DEB_STAGE}/usr/bin/${PROJECT_NAME}" completions zsh > "${DEB_STAGE}/usr/share/zsh/site-functions/_${PROJECT_NAME}" 2>/dev/null || true
"${DEB_STAGE}/usr/bin/${PROJECT_NAME}" completions fish > "${DEB_STAGE}/usr/share/fish/vendor_completions.d/${PROJECT_NAME}.fish" 2>/dev/null || true

dpkg-deb --build --root-owner-group "${DEB_STAGE}" "${DIST_DIR}/${DEB_PKG_NAME}"
echo -e "${GREEN}✓ Created Debian package: ${DIST_DIR}/${DEB_PKG_NAME}${NC}"

# ------------------------------------------------------------------------------
# 4. Assemble Portable Release Tarball
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}[4/6] Assembling portable release tarball...${NC}"
TAR_STAGE="${DIST_DIR}/${PKG_BASENAME}"
rm -rf "${TAR_STAGE}"
mkdir -p "${TAR_STAGE}/bin"
mkdir -p "${TAR_STAGE}/systemd"
mkdir -p "${TAR_STAGE}/docs"
mkdir -p "${TAR_STAGE}/completions"

cp "${RELEASE_BIN}" "${TAR_STAGE}/bin/${PROJECT_NAME}"
strip "${TAR_STAGE}/bin/${PROJECT_NAME}"
chmod 755 "${TAR_STAGE}/bin/${PROJECT_NAME}"

if [ -f "${WORKSPACE_ROOT}/packaging/systemd/oxide-watch.service" ]; then
    cp "${WORKSPACE_ROOT}/packaging/systemd/oxide-watch.service" "${TAR_STAGE}/systemd/"
fi
if [ -f "${WORKSPACE_ROOT}/scripts/install.sh" ]; then
    cp "${WORKSPACE_ROOT}/scripts/install.sh" "${TAR_STAGE}/install.sh"
    chmod +x "${TAR_STAGE}/install.sh"
fi

cp "${WORKSPACE_ROOT}/README.md" "${TAR_STAGE}/docs/"
cp "${WORKSPACE_ROOT}/CHANGELOG.md" "${TAR_STAGE}/docs/" 2>/dev/null || true
cp "${WORKSPACE_ROOT}/LICENSE-MIT" "${TAR_STAGE}/" 2>/dev/null || true
cp "${WORKSPACE_ROOT}/LICENSE-APACHE" "${TAR_STAGE}/" 2>/dev/null || true

# Generate completions into tarball
"${TAR_STAGE}/bin/${PROJECT_NAME}" completions bash > "${TAR_STAGE}/completions/oxide-embed.bash" 2>/dev/null || true
"${TAR_STAGE}/bin/${PROJECT_NAME}" completions zsh > "${TAR_STAGE}/completions/_oxide-embed" 2>/dev/null || true
"${TAR_STAGE}/bin/${PROJECT_NAME}" completions fish > "${TAR_STAGE}/completions/oxide-embed.fish" 2>/dev/null || true

tar -czf "${DIST_DIR}/${TARBALL_NAME}" -C "${DIST_DIR}" "${PKG_BASENAME}"
rm -rf "${TAR_STAGE}" "${BUILD_DIR}"
echo -e "${GREEN}✓ Created portable tarball: ${DIST_DIR}/${TARBALL_NAME}${NC}"

# ------------------------------------------------------------------------------
# 5. Generate Standalone Binary Distribution Link & Desktop Launcher
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}[5/6] Preparing standalone binary and desktop entry...${NC}"
mkdir -p "${DIST_DIR}/bin"
cp "${RELEASE_BIN}" "${DIST_DIR}/bin/${PROJECT_NAME}"
strip "${DIST_DIR}/bin/${PROJECT_NAME}"
chmod 755 "${DIST_DIR}/bin/${PROJECT_NAME}"

# Generate Desktop Entry for IDE / Terminal App integration
cat << EOF > "${DIST_DIR}/${PROJECT_NAME}.desktop"
[Desktop Entry]
Name=Oxide-Embed
Comment=Ultra-fast local-first code graph indexing, cognitive memory & MCP engine
Exec=oxide-embed mcp
Icon=utilities-terminal
Terminal=true
Type=Application
Categories=Development;Utility;
Keywords=rust;ast;indexing;mcp;rag;embeddings;
EOF

# ------------------------------------------------------------------------------
# 6. Generate Cryptographic Checksums (SHA256)
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}[6/6] Generating SHA256 checksums...${NC}"
cd "${DIST_DIR}"
sha256sum "${DEB_PKG_NAME}" "${TARBALL_NAME}" "bin/${PROJECT_NAME}" > SHA256SUMS
echo -e "${GREEN}✓ SHA256SUMS generated${NC}"

# ------------------------------------------------------------------------------
# Build Summary
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}${BOLD}==============================================================================${NC}"
echo -e "${GREEN}${BOLD}✓ Oxide-Embed Ubuntu Application Build Completed Successfully!${NC}"
echo -e "=============================================================================="
echo -e "📦 Output Artifacts in: ${CYAN}${DIST_DIR}/${NC}"
ls -lh "${DIST_DIR}/${DEB_PKG_NAME}" "${DIST_DIR}/${TARBALL_NAME}" "${DIST_DIR}/bin/${PROJECT_NAME}" "${DIST_DIR}/SHA256SUMS"
echo -e "------------------------------------------------------------------------------"
echo -e "👉 Quick Installation on Ubuntu / Debian / Pop!_OS:"
echo -e "   ${CYAN}sudo dpkg -i dist/${DEB_PKG_NAME}${NC}"
echo -e "   or install standalone binary:"
echo -e "   ${CYAN}cp dist/bin/oxide-embed ~/.local/bin/${NC}"
echo -e "${BLUE}${BOLD}==============================================================================${NC}"
