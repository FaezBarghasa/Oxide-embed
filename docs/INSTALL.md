# Oxide-embed Linux Installation Guide

Oxide-embed provides first-class native packages and standalone binaries for all major Linux distributions, optimized for fast installation and automated systemd user daemon management.

---

## 1. Quick Universal Installation (Recommended)

Run the automated installer script to detect your CPU architecture, install the binary to your path, and optionally configure the systemd file watcher daemon:

```bash
curl -fsSL https://raw.githubusercontent.com/FaezBarghasa/Oxide-embed/main/scripts/install.sh | bash
```

To install directly from a cloned repository:

```bash
git clone https://github.com/FaezBarghasa/Oxide-embed.git
cd Oxide-embed
./scripts/install.sh --enable-service
```

---

## 2. Debian / Ubuntu / Pop!_OS (`.deb` Package)

Native Debian packages are provided for `amd64` and `arm64` architectures.

### Option A: Install Prebuilt `.deb`
```bash
# Download and install with dpkg or apt
sudo dpkg -i dist/oxide-embed_0.2.0_amd64.deb

# Or install with auto-resolved dependencies
sudo apt install ./dist/oxide-embed_0.2.0_amd64.deb
```

### Option B: Build `.deb` from Source
Ensure `cargo`, `rustc >= 1.85`, and `dpkg-deb` are installed:
```bash
./scripts/package-deb.sh
sudo dpkg -i dist/oxide-embed_0.2.0_amd64.deb
```

---

## 3. Arch Linux / Manjaro (`PKGBUILD`)

Install via standard Arch Linux `makepkg` workflow:

```bash
cd packaging/arch
makepkg -si
```

This compiles the binary with native CPU optimizations and places `oxide-embed` into `/usr/bin/` with default systemd user unit files.

---

## 4. Fedora / RHEL / Rocky Linux (RPM)

Build and install an RPM package using `rpmbuild`:

```bash
# Prepare rpmbuild directory tree
mkdir -p ~/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
cp packaging/fedora/oxide-embed.spec ~/rpmbuild/SPECS/

# Build RPM
rpmbuild -ba ~/rpmbuild/SPECS/oxide-embed.spec

# Install the generated RPM
sudo dnf install ~/rpmbuild/RPMS/x86_64/oxide-embed-0.2.0-1.fc*.x86_64.rpm
```

---

## 5. Alpine Linux (`APKBUILD`)

To build on Alpine Linux with musl libc:

```bash
cd packaging/alpine
abuild -r
apk add ~/packages/*/x86_64/oxide-embed-0.2.0-r0.apk
```

---

## 6. Systemd Background Watcher Daemon

Oxide-embed can monitor your codebases continuously in the background, updating AST symbol graphs and vector indexes incrementally whenever files change:

```bash
# Enable and start the user service
systemctl --user enable --now oxide-watch.service

# Check live daemon status
systemctl --user status oxide-watch.service

# Stream logs
journalctl --user -u oxide-watch.service -f
```

---

## 7. Verification

Verify that the CLI and all subcommands are operational:

```bash
oxide-embed --version
oxide-embed --help
oxide-embed check-drift
oxide-embed mcp-status
```
