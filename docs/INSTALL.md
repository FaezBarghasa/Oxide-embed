# Oxide-Embed Cross-Platform Installation & Build Guide

Oxide-Embed provides native release builders, one-liner installers, and distribution packages for Linux, macOS, and Windows.

---

## 1. Quick One-Liner Installers

### Linux (Debian, Ubuntu, Pop!_OS, Arch, Fedora, Alpine)
```bash
# From local repository:
./install.sh

# Or from remote repository:
curl -fsSL https://raw.githubusercontent.com/FaezBarghasa/Oxide-embed/main/install.sh | bash
```

### Windows (PowerShell 5.1+ & PowerShell Core)
```powershell
# From local repository:
.\install.ps1

# Or with custom target directory & desktop icon:
.\install.ps1 -InstallDir "$env:LOCALAPPDATA\Programs\Oxide-Embed\bin" -CreateDesktopShortcut
```

---

## 2. Full Application Builder Pipelines

Dedicated, self-contained shell and PowerShell scripts automate compiling, stripping, creating desktop entries, packaging, and generating SHA256 checksums into `dist/`.

### A. Ubuntu / Debian / Pop!_OS (`build-ubuntu-app.sh`)
```bash
./build-ubuntu-app.sh
```
**Artifacts Generated in `dist/`:**
- **Debian Package**: `dist/oxide-embed_0.4.0_amd64.deb`
- **Portable Tarball**: `dist/oxide-embed-v0.4.0-x86_64-unknown-linux-gnu.tar.gz`
- **Standalone Stripped Binary**: `dist/bin/oxide-embed`
- **Desktop Entry**: `dist/oxide-embed.desktop`
- **Checksums**: `dist/SHA256SUMS`

**Install Debian Package:**
```bash
sudo dpkg -i dist/oxide-embed_0.4.0_amd64.deb
```

---

### B. macOS Universal Application (`build-macos-app.sh`)
```bash
./build-macos-app.sh
```
**Artifacts Generated in `dist/macos/`:**
- **Universal Binary (`lipo`)**: `dist/macos/bin/oxide-embed` (Apple Silicon + Intel)
- **Universal Release Tarball**: `dist/macos/oxide-embed-v0.4.0-universal-apple-darwin.tar.gz`
- **LaunchAgent Service Plist**: `dist/macos/launchd/com.faezbarghasa.oxide-embed.plist`
- **Homebrew Formula**: `dist/macos/homebrew/oxide-embed.rb`
- **Checksums**: `dist/macos/SHA256SUMS`

**Load macOS Background Service:**
```bash
cp dist/macos/launchd/com.faezbarghasa.oxide-embed.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/com.faezbarghasa.oxide-embed.plist
```

---

### C. Windows Applications (`build-windows-app.sh` & `build-windows-app.ps1`)

#### Cross-Compilation from Linux:
```bash
# Requires MinGW or cargo-xwin:
./build-windows-app.sh
```

#### Native Build on Windows (PowerShell):
```powershell
.\build-windows-app.ps1
```

**Artifacts Generated in `dist/windows/`:**
- **Executable**: `dist/windows/bin/oxide-embed.exe`
- **Release Zip Archive**: `dist/windows/oxide-embed-v0.4.0-x86_64-pc-windows-msvc.zip`
- **PowerShell Installer**: `dist/windows/oxide-embed-v0.4.0-x86_64-pc-windows-msvc/install.ps1`
- **MCP Launcher**: `dist/windows/oxide-embed-v0.4.0-x86_64-pc-windows-msvc/run-mcp.bat`
- **Checksums**: `dist/windows/SHA256SUMS`

---

## 3. Native Linux Distro Packages

### Arch Linux / Manjaro (`PKGBUILD`)
```bash
cd packaging/arch
makepkg -si
```

### Fedora / RHEL / Rocky Linux (RPM)
```bash
mkdir -p ~/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
cp packaging/fedora/oxide-embed.spec ~/rpmbuild/SPECS/
rpmbuild -ba ~/rpmbuild/SPECS/oxide-embed.spec
sudo dnf install ~/rpmbuild/RPMS/x86_64/oxide-embed-*.rpm
```

### Alpine Linux (`APKBUILD`)
```bash
cd packaging/alpine
abuild -r
apk add ~/packages/*/x86_64/oxide-embed-*.apk
```

---

## 4. Background Watcher Daemon

Oxide-Embed monitors codebases continuously in the background, updating AST symbol graphs and vector indexes incrementally on file save:

### Linux (Systemd)
```bash
# Enable & start user service
systemctl --user enable --now oxide-watch.service

# Inspect live status and logs
systemctl --user status oxide-watch.service
journalctl --user -u oxide-watch.service -f
```

### macOS (LaunchAgent)
```bash
launchctl load ~/Library/LaunchAgents/com.faezbarghasa.oxide-embed.plist
tail -f /tmp/oxide-embed.stdout.log
```

---

## 5. Verification

```bash
oxide-embed --version
oxide-embed --help
oxide-embed mcp-status
```
