#!/usr/bin/env bash
# ==============================================================================
# Debian / Ubuntu / Pop!_OS Package Builder (.deb)
# ==============================================================================
set -euo pipefail

VERSION="0.2.0"
PACKAGE_NAME="oxide-embed"
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64|amd64)
        DEB_ARCH="amd64"
        ;;
    aarch64|arm64)
        DEB_ARCH="arm64"
        ;;
    *)
        DEB_ARCH="amd64"
        ;;
esac

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${PROJECT_ROOT}/dist"
BUILD_DIR="${PROJECT_ROOT}/target/deb-build"
DEB_DIR="${BUILD_DIR}/${PACKAGE_NAME}_${VERSION}_${DEB_ARCH}"

echo "📦 Building Debian package: ${PACKAGE_NAME}_${VERSION}_${DEB_ARCH}.deb"
mkdir -p "${DIST_DIR}"
rm -rf "${BUILD_DIR}"
mkdir -p "${DEB_DIR}/DEBIAN"
mkdir -p "${DEB_DIR}/usr/bin"
mkdir -p "${DEB_DIR}/usr/lib/systemd/user"
mkdir -p "${DEB_DIR}/usr/share/doc/${PACKAGE_NAME}"
mkdir -p "${DEB_DIR}/usr/share/bash-completion/completions"
mkdir -p "${DEB_DIR}/usr/share/zsh/site-functions"
mkdir -p "${DEB_DIR}/usr/share/fish/vendor_completions.d"

# 1. Compile Release Binary
echo "🔨 Compiling release binary with LTO..."
(cd "${PROJECT_ROOT}" && cargo build --release -p oxide-cli)
cp "${PROJECT_ROOT}/target/release/oxide-embed" "${DEB_DIR}/usr/bin/oxide-embed"
strip "${DEB_DIR}/usr/bin/oxide-embed"

# 2. Copy Control Files & Metadata
cp "${PROJECT_ROOT}/packaging/debian/control" "${DEB_DIR}/DEBIAN/control"
# Update architecture in control file
sed -i "s/Architecture: .*/Architecture: ${DEB_ARCH}/" "${DEB_DIR}/DEBIAN/control"
cp "${PROJECT_ROOT}/packaging/debian/postinst" "${DEB_DIR}/DEBIAN/postinst"
cp "${PROJECT_ROOT}/packaging/debian/prerm" "${DEB_DIR}/DEBIAN/prerm"
chmod 755 "${DEB_DIR}/DEBIAN/postinst" "${DEB_DIR}/DEBIAN/prerm"

# 3. Copy Docs & Licenses
cp "${PROJECT_ROOT}/README.md" "${DEB_DIR}/usr/share/doc/${PACKAGE_NAME}/README.md"
cp "${PROJECT_ROOT}/CHANGELOG.md" "${DEB_DIR}/usr/share/doc/${PACKAGE_NAME}/CHANGELOG.md"
cp "${PROJECT_ROOT}/packaging/debian/copyright" "${DEB_DIR}/usr/share/doc/${PACKAGE_NAME}/copyright"
cp "${PROJECT_ROOT}/packaging/systemd/oxide-watch.service" "${DEB_DIR}/usr/lib/systemd/user/oxide-watch.service"

# 4. Build Package
echo "📦 Packing with dpkg-deb..."
dpkg-deb --build --root-owner-group "${DEB_DIR}" "${DIST_DIR}/${PACKAGE_NAME}_${VERSION}_${DEB_ARCH}.deb"

echo "✅ Created: ${DIST_DIR}/${PACKAGE_NAME}_${VERSION}_${DEB_ARCH}.deb"
ls -lh "${DIST_DIR}/${PACKAGE_NAME}_${VERSION}_${DEB_ARCH}.deb"
