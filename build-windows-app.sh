#!/usr/bin/env bash
# ==============================================================================
# Oxide-Embed Windows Full Application Cross-Compilation & Packaging Script
# Target: x86_64-pc-windows-gnu (MinGW-w64)
# Outputs: oxide-embed.exe, portable zip bundle, powershell install scripts, SHA256
# ==============================================================================
set -euo pipefail

BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

PROJECT_NAME="oxide-embed"
VERSION="0.4.0"
TARGET_TRIPLE="x86_64-pc-windows-gnu"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="${SCRIPT_DIR}"
DIST_DIR="${WORKSPACE_ROOT}/dist"
WIN_DIST_DIR="${DIST_DIR}/windows"
ZIP_BASENAME="${PROJECT_NAME}-v${VERSION}-${TARGET_TRIPLE}"
STAGE_DIR="${WIN_DIST_DIR}/${ZIP_BASENAME}"

echo -e "${CYAN}${BOLD}"
cat << "EOF"
  ____       _     _             _____                 _             _ 
 / __ \     (_)   | |           |  ___|               | |           | |
| |  | |_  ___  __| | ___ ______| |__   _ __ ___  __ _| |__   ___  __| |
| |  | \ \/ / |/ _` |/ _ \______|  __| | '_ ` _ \/ _` | '_ \ / _ \/ _` |
| |__| |>  <| | (_| |  __/      | |___ | | | | | | (_| | |_) |  __/ (_| |
 \____//_/\_\_|\__,_|\___|      \____/ |_| |_| |_|\__,_|_.__/ \___|\__,_|
EOF
echo -e "${NC}"
echo -e "${CYAN}${BOLD}⚡ Oxide-Embed Windows Full Application Builder (v${VERSION})${NC}"
echo -e "   Target: ${GREEN}${TARGET_TRIPLE}${NC}"
echo -e "=============================================================================="

# ------------------------------------------------------------------------------
# 1. Dependency Validation & Rust Target
# ------------------------------------------------------------------------------
USE_XWIN=0

if command -v cargo-xwin >/dev/null 2>&1; then
    USE_XWIN=1
    TARGET_TRIPLE="x86_64-pc-windows-msvc"
    echo -e "${GREEN}✓ Detected cargo-xwin! Using MSVC toolchain with automatic SDK headers.${NC}"
    
    # Check for NASM and Clang needed by crypto/C crates (e.g., aws-lc-sys, surrealdb)
    if ! command -v nasm >/dev/null 2>&1; then
        echo -e "${YELLOW}Notice: 'nasm' assembler is required by low-level crypto dependencies (aws-lc-sys).${NC}"
        echo -e "${YELLOW}Please install nasm: ${CYAN}sudo apt-get install -y nasm clang lld${NC}"
        exit 1
    fi
    rustup target add "${TARGET_TRIPLE}"
elif command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    TARGET_TRIPLE="x86_64-pc-windows-gnu"
    echo -e "${GREEN}✓ Detected MinGW-w64! Using GNU toolchain.${NC}"
    rustup target add "${TARGET_TRIPLE}"
else
    echo -e "${YELLOW}Notice: Neither cargo-xwin nor x86_64-w64-mingw32-gcc found on system.${NC}"
    echo -e "${YELLOW}To enable Windows cross-compilation, run:${NC}"
    echo -e "   ${CYAN}cargo install cargo-xwin && sudo apt-get install -y nasm clang lld${NC}"
    echo -e "   or install MinGW: ${CYAN}sudo apt-get install -y gcc-mingw-w64-x86-64 binutils-mingw-w64-x86-64${NC}"
    exit 1
fi

ZIP_BASENAME="${PROJECT_NAME}-v${VERSION}-${TARGET_TRIPLE}"
STAGE_DIR="${WIN_DIST_DIR}/${ZIP_BASENAME}"

# ------------------------------------------------------------------------------
# 2. Compile Windows Release Executable
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}[2/5] Compiling Oxide-Embed for Windows (${TARGET_TRIPLE})...${NC}"
cd "${WORKSPACE_ROOT}"
if [ "${USE_XWIN}" -eq 1 ]; then
    cargo xwin build --release --target "${TARGET_TRIPLE}" -p oxide-cli
else
    cargo build --release --target "${TARGET_TRIPLE}" -p oxide-cli
fi

WIN_BIN="${WORKSPACE_ROOT}/target/${TARGET_TRIPLE}/release/${PROJECT_NAME}.exe"
if [ ! -f "${WIN_BIN}" ]; then
    echo -e "${RED}❌ Release binary not found at ${WIN_BIN}${NC}" >&2
    exit 1
fi

# Strip debug symbols using MinGW strip if available
if command -v x86_64-w64-mingw32-strip >/dev/null 2>&1; then
    x86_64-w64-mingw32-strip "${WIN_BIN}"
fi

BIN_SIZE_RAW="$(stat -c%s "${WIN_BIN}" 2>/dev/null || stat -f%z "${WIN_BIN}" 2>/dev/null || echo 0)"
BIN_SIZE_MB="$(awk "BEGIN {printf \"%.2f\", ${BIN_SIZE_RAW}/1048576}")"
echo -e "${GREEN}✓ Compiled Windows executable: ${WIN_BIN} (${BIN_SIZE_MB} MB)${NC}"

# ------------------------------------------------------------------------------
# 3. Assemble Windows Distribution Bundle
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}[3/5] Assembling Windows release bundle directory...${NC}"
mkdir -p "${WIN_DIST_DIR}"
rm -rf "${STAGE_DIR}"
mkdir -p "${STAGE_DIR}/bin"
mkdir -p "${STAGE_DIR}/docs"

cp "${WIN_BIN}" "${STAGE_DIR}/bin/${PROJECT_NAME}.exe"
cp "${WORKSPACE_ROOT}/README.md" "${STAGE_DIR}/docs/README.md"
cp "${WORKSPACE_ROOT}/CHANGELOG.md" "${STAGE_DIR}/docs/CHANGELOG.md" 2>/dev/null || true
cp "${WORKSPACE_ROOT}/LICENSE-MIT" "${STAGE_DIR}/LICENSE-MIT.txt" 2>/dev/null || true
cp "${WORKSPACE_ROOT}/LICENSE-APACHE" "${STAGE_DIR}/LICENSE-APACHE.txt" 2>/dev/null || true

# Standalone quick installer for Windows PowerShell
cat << 'EOF' > "${STAGE_DIR}/install.ps1"
# Oxide-Embed Windows User-Level Installer
$ErrorActionPreference = "Stop"
$InstallDir = "$env:LOCALAPPDATA\Programs\Oxide-Embed\bin"
Write-Host "⚡ Installing Oxide-Embed into $InstallDir..." -ForegroundColor Cyan

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item -Path "$PSScriptRoot\bin\oxide-embed.exe" -Destination "$InstallDir\oxide-embed.exe" -Force

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "✓ Added $InstallDir to User PATH." -ForegroundColor Green
} else {
    Write-Host "✓ $InstallDir already present in PATH." -ForegroundColor Green
}

Write-Host "✅ Oxide-Embed installed successfully! Restart your terminal and run 'oxide-embed --help'." -ForegroundColor Green
EOF

# Standalone runner batch script
cat << 'EOF' > "${STAGE_DIR}/run-mcp.bat"
@echo off
setlocal
set "BIN_DIR=%~dp0bin"
"%BIN_DIR%\oxide-embed.exe" mcp %*
EOF

# ------------------------------------------------------------------------------
# 4. Generate Zip Archive
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}[4/5] Packaging Zip archive...${NC}"
ZIP_PATH="${WIN_DIST_DIR}/${ZIP_BASENAME}.zip"
rm -f "${ZIP_PATH}"

if command -v zip >/dev/null 2>&1; then
    (cd "${WIN_DIST_DIR}" && zip -r -9 "${ZIP_BASENAME}.zip" "${ZIP_BASENAME}")
elif command -v 7z >/dev/null 2>&1; then
    (cd "${WIN_DIST_DIR}" && 7z a -tzip -mx=9 "${ZIP_BASENAME}.zip" "${ZIP_BASENAME}")
fi

# Copy standalone binary to dist/windows/bin/
mkdir -p "${WIN_DIST_DIR}/bin"
cp "${WIN_BIN}" "${WIN_DIST_DIR}/bin/${PROJECT_NAME}.exe"

# ------------------------------------------------------------------------------
# 5. Generate Checksums
# ------------------------------------------------------------------------------
echo -e "\n${BLUE}[5/5] Generating SHA256 checksums...${NC}"
cd "${WIN_DIST_DIR}"
sha256sum "${ZIP_BASENAME}.zip" "bin/${PROJECT_NAME}.exe" > SHA256SUMS

echo -e "\n${CYAN}${BOLD}==============================================================================${NC}"
echo -e "${GREEN}${BOLD}✓ Oxide-Embed Windows Build & Packaging Completed Successfully!${NC}"
echo -e "=============================================================================="
echo -e "📦 Output Artifacts in: ${CYAN}${WIN_DIST_DIR}/${NC}"
ls -lh "${WIN_DIST_DIR}/${ZIP_BASENAME}.zip" "${WIN_DIST_DIR}/bin/${PROJECT_NAME}.exe" "${WIN_DIST_DIR}/SHA256SUMS"
echo -e "------------------------------------------------------------------------------"
echo -e "👉 PowerShell Deployment:"
echo -e "   Extract ${ZIP_BASENAME}.zip and execute ${CYAN}.\\install.ps1${NC}"
echo -e "${CYAN}${BOLD}==============================================================================${NC}"
