<#
.SYNOPSIS
    Oxide-Embed Native Windows Build & Packaging Script
.DESCRIPTION
    Builds the oxide-embed binary on Windows PowerShell and packages the release zip archive.
#>

$ErrorActionPreference = "Stop"

$ProjectName = "oxide-embed"
$Version = "0.4.0"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$WorkspaceRoot = $ScriptDir
$DistDir = Join-Path $WorkspaceRoot "dist\windows"
$ZipBasename = "$ProjectName-v$Version-x86_64-pc-windows-msvc"
$StageDir = Join-Path $DistDir $ZipBasename

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  Oxide-Embed Native Windows Builder (v$Version)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Compile Release Binary
Write-Host "`n[1/4] Compiling Oxide-Embed release binary..." -ForegroundColor Yellow
Set-Location $WorkspaceRoot
cargo build --release -p oxide-cli

$ReleaseBin = Join-Path $WorkspaceRoot "target\release\$ProjectName.exe"
if (-not (Test-Path $ReleaseBin)) {
    Write-Error "[-] Release binary not found at $ReleaseBin"
}
Write-Host "✓ Compiled: $ReleaseBin" -ForegroundColor Green

# 2. Stage Distribution Bundle
Write-Host "`n[2/4] Staging distribution files..." -ForegroundColor Yellow
if (Test-Path $StageDir) { Remove-Item -Recurse -Force $StageDir }
New-Item -ItemType Directory -Force -Path (Join-Path $StageDir "bin") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $StageDir "docs") | Out-Null

Copy-Item $ReleaseBin -Destination (Join-Path $StageDir "bin\$ProjectName.exe")
Copy-Item (Join-Path $WorkspaceRoot "README.md") -Destination (Join-Path $StageDir "docs\README.md")

if (Test-Path (Join-Path $WorkspaceRoot "CHANGELOG.md")) {
    Copy-Item (Join-Path $WorkspaceRoot "CHANGELOG.md") -Destination (Join-Path $StageDir "docs\CHANGELOG.md")
}

# Add Installer
$InstallerContent = @'
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
'@
Set-Content -Path (Join-Path $StageDir "install.ps1") -Value $InstallerContent

# 3. Create Zip Package
Write-Host "`n[3/4] Creating zip package..." -ForegroundColor Yellow
$ZipPath = Join-Path $DistDir "$ZipBasename.zip"
if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
Compress-Archive -Path "$StageDir\*" -DestinationPath $ZipPath

# Copy standalone binary
New-Item -ItemType Directory -Force -Path (Join-Path $DistDir "bin") | Out-Null
Copy-Item $ReleaseBin -Destination (Join-Path $DistDir "bin\$ProjectName.exe")

# 4. Generate SHA256 Checksum
Write-Host "`n[4/4] Calculating SHA256 checksum..." -ForegroundColor Yellow
$Hash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash
Set-Content -Path (Join-Path $DistDir "SHA256SUMS") -Value "$Hash  $ZipBasename.zip"

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host "✓ Windows Build & Packaging Complete!" -ForegroundColor Green
Write-Host "  Zip Package: $ZipPath" -ForegroundColor Cyan
Write-Host "  Standalone:  $(Join-Path $DistDir "bin\$ProjectName.exe")" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Green
