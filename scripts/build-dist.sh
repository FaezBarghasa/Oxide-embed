#!/usr/bin/env bash
# ==============================================================================
# Oxide-embed Master Release Distribution Builder
# Packages tarballs, Debian packages, and generates SHA256 checksums
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${WORKSPACE_ROOT}/dist"
VERSION="0.2.0"
TARGET_ARCH="$(uname -m)"
TARGET_TRIPLE="${TARGET_ARCH}-unknown-linux-gnu"
PKG_BASENAME="oxide-embed-v${VERSION}-${TARGET_TRIPLE}"
STAGE_DIR="${DIST_DIR}/${PKG_BASENAME}"

echo "=========================================================="
echo "  Oxide-embed Release Distribution Builder v${VERSION}"
echo "  Architecture: ${TARGET_ARCH}"
echo "=========================================================="

mkdir -p "${DIST_DIR}"

echo "==> Step 1: Building optimized release binary..."
cargo build --manifest-path "${WORKSPACE_ROOT}/Cargo.toml" --release --bin oxide-embed

RELEASE_BIN="${WORKSPACE_ROOT}/target/release/oxide-embed"
if [[ ! -f "${RELEASE_BIN}" ]]; then
    echo "[-] Error: Release binary not found at ${RELEASE_BIN}" >&2
    exit 1
fi

echo "==> Step 2: Building Debian (.deb) package..."
"${SCRIPT_DIR}/package-deb.sh"

echo "==> Step 3: Assembling portable tarball archive..."
rm -rf "${STAGE_DIR}"
mkdir -p "${STAGE_DIR}/bin"
mkdir -p "${STAGE_DIR}/systemd"
mkdir -p "${STAGE_DIR}/docs"

cp "${RELEASE_BIN}" "${STAGE_DIR}/bin/oxide-embed"
strip "${STAGE_DIR}/bin/oxide-embed"

cp "${WORKSPACE_ROOT}/packaging/systemd/oxide-watch.service" "${STAGE_DIR}/systemd/"
cp "${WORKSPACE_ROOT}/scripts/install.sh" "${STAGE_DIR}/install.sh"
chmod +x "${STAGE_DIR}/install.sh"
cp "${WORKSPACE_ROOT}/README.md" "${STAGE_DIR}/"
cp "${WORKSPACE_ROOT}/CHANGELOG.md" "${STAGE_DIR}/"

TARBALL_PATH="${DIST_DIR}/${PKG_BASENAME}.tar.gz"
tar -czf "${TARBALL_PATH}" -C "${DIST_DIR}" "${PKG_BASENAME}"
rm -rf "${STAGE_DIR}"

echo "[+] Created portable tarball: ${TARBALL_PATH}"

echo "==> Step 4: Generating SHA256 checksums..."
cd "${DIST_DIR}"
sha256sum oxide-embed* > SHA256SUMS

echo "=========================================================="
echo "  Distribution Build Completed Successfully!"
echo "  Artifacts located in: ${DIST_DIR}"
ls -lh "${DIST_DIR}"
echo "=========================================================="
