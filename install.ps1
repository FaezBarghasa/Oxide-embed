<#
.SYNOPSIS
    Oxide-Embed Universal Windows PowerShell Installer
.DESCRIPTION
    Installs Oxide-Embed for Windows, configures User PATH, creates desktop shortcuts,
    sets up shell completions, and verifies execution.
.EXAMPLE
    .\install.ps1
    irm https://raw.githubusercontent.com/FaezBarghasa/Oxide-embed/main/install.ps1 | iex
#>

param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Oxide-Embed\bin",
    [switch]$CreateDesktopShortcut,
    [switch]$ForceRebuild
)

$ErrorActionPreference = "Stop"
$ProjectName = "oxide-embed"
$Version = "0.4.0"
$Repo = "FaezBarghasa/Oxide-embed"

Write-Host @"
  ____       _     _             _____                 _             _ 
 / __ \     (_)   | |           |  ___|               | |           | |
| |  | |_  ___  __| | ___ ______| |__   _ __ ___  __ _| |__   ___  __| |
| |  | \ \/ / |/ _` |/ _ \______|  __| | '_ ` _ \/ _` | '_ \ / _ \/ _` |
| |__| |>  <| | (_| |  __/      | |___ | | | | | | (_| | |_) |  __/ (_| |
 \____//_/\_\_|\__,_|\___|      \____/ |_| |_| |_|\__,_|_.__/ \___|\__,_|

"@ -ForegroundColor Cyan

Write-Host "⚡ Oxide-Embed Windows Installer (v$Version)" -ForegroundColor Cyan
Write-Host "   Repository: https://github.com/$Repo" -ForegroundColor DarkGray
Write-Host "==============================================================================" -ForegroundColor DarkGray

# 1. Target Directory Preparation
Write-Host "`n[1/5] Preparing installation directory..." -ForegroundColor Yellow
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}
Write-Host "✓ Target directory: $InstallDir" -ForegroundColor Green

# 2. Binary Discovery & Placement
Write-Host "`n[2/5] Locating / Compiling $ProjectName executable..." -ForegroundColor Yellow

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$CandidatePaths = @(
    (Join-Path $ScriptDir "bin\$ProjectName.exe"),
    (Join-Path $ScriptDir "dist\windows\bin\$ProjectName.exe"),
    (Join-Path $ScriptDir "target\release\$ProjectName.exe"),
    (Join-Path $ScriptDir "target\x86_64-pc-windows-msvc\release\$ProjectName.exe"),
    (Join-Path $ScriptDir "target\x86_64-pc-windows-gnu\release\$ProjectName.exe"),
    (Join-Path $ScriptDir "$ProjectName.exe")
)

$SourceBin = $null
foreach ($path in $CandidatePaths) {
    if (Test-Path $path) {
        $SourceBin = $path
        break
    }
}

$DestBin = Join-Path $InstallDir "$ProjectName.exe"

if ($SourceBin -and (-not $ForceRebuild)) {
    Write-Host "⚙️  Using existing binary: $SourceBin" -ForegroundColor DarkCyan
    Copy-Item -Path $SourceBin -Destination $DestBin -Force
} else {
    Write-Host "🔨 Prebuilt binary not found. Compiling via Cargo (release profile)..." -ForegroundColor DarkCyan
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Push-Location $ScriptDir
        cargo build --release -p oxide-cli
        Pop-Location
        $BuiltBin = Join-Path $ScriptDir "target\release\$ProjectName.exe"
        if (Test-Path $BuiltBin) {
            Copy-Item -Path $BuiltBin -Destination $DestBin -Force
        } else {
            Write-Error "[-] Compilation failed: $BuiltBin not created."
        }
    } else {
        Write-Error "[-] Neither prebuilt binary nor Cargo found. Please install Rust or download prebuilt release."
    }
}

Write-Host "✓ Binary installed: $DestBin" -ForegroundColor Green

# 3. Environment & PATH Configuration
Write-Host "`n[3/5] Configuring User PATH..." -ForegroundColor Yellow
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "✓ Added $InstallDir to User Environment PATH." -ForegroundColor Green
} else {
    Write-Host "✓ $InstallDir is already in User PATH." -ForegroundColor Green
}

# 4. PowerShell Auto-Completions
Write-Host "`n[4/5] Configuring PowerShell auto-completions..." -ForegroundColor Yellow
try {
    $ProfileDir = Split-Path -Parent $PROFILE
    if (-not (Test-Path $ProfileDir)) {
        New-Item -ItemType Directory -Force -Path $ProfileDir | Out-Null
    }
    
    $CompletionScript = Join-Path $InstallDir "oxide-embed-completion.ps1"
    & "$DestBin" completions powershell | Out-File -FilePath $CompletionScript -Encoding utf8 -Force
    
    $ProfileEntry = "if (Test-Path '$CompletionScript') { . '$CompletionScript' }"
    if (Test-Path $PROFILE) {
        $CurrentProfile = Get-Content $PROFILE -Raw
        if ($CurrentProfile -notlike "*oxide-embed-completion.ps1*") {
            Add-Content -Path $PROFILE -Value "`n$ProfileEntry"
            Write-Host "✓ Added auto-completions to $PROFILE" -ForegroundColor Green
        }
    } else {
        Set-Content -Path $PROFILE -Value "$ProfileEntry"
        Write-Host "✓ Created profile with completions at $PROFILE" -ForegroundColor Green
    }
} catch {
    Write-Host "Notice: Could not set up profile completions automatically ($_)." -ForegroundColor DarkGray
}

# 5. Standalone Runners & Desktop Shortcut
Write-Host "`n[5/5] Generating helper shortcuts..." -ForegroundColor Yellow

# Helper batch file
$BatchPath = Join-Path $InstallDir "oxide-mcp.cmd"
@"
@echo off
"%~dp0oxide-embed.exe" mcp %*
"@ | Set-Content -Path $BatchPath -Encoding ASCII
Write-Host "✓ Created quick launcher: $BatchPath" -ForegroundColor Green

if ($CreateDesktopShortcut) {
    try {
        $WshShell = New-Object -ComObject WScript.Shell
        $ShortcutPath = Join-Path ([Environment]::GetFolderPath("Desktop")) "Oxide-Embed MCP.lnk"
        $Shortcut = $WshShell.CreateShortcut($ShortcutPath)
        $Shortcut.TargetPath = $DestBin
        $Shortcut.Arguments = "mcp"
        $Shortcut.Description = "Oxide-Embed Local MCP Graph Engine"
        $Shortcut.Save()
        Write-Host "✓ Created Desktop Shortcut: $ShortcutPath" -ForegroundColor Green
    } catch {
        Write-Host "Notice: Desktop shortcut skipped." -ForegroundColor DarkGray
    }
}

# 6. Verification and Summary
Write-Host "`n==============================================================================" -ForegroundColor Green
Write-Host "✅ Oxide-Embed (v$Version) successfully installed to:" -ForegroundColor Green
Write-Host "   $DestBin" -ForegroundColor Cyan
Write-Host "==============================================================================" -ForegroundColor Green

Write-Host "`n👉 Verify installation (in a new PowerShell window):"
Write-Host "   oxide-embed --version" -ForegroundColor Cyan

Write-Host "`n👉 Core Commands:"
Write-Host "   oxide-embed init             # Initialize local .oxide memory" -ForegroundColor Cyan
Write-Host "   oxide-embed index            # Index AST, symbols, and knowledge graph" -ForegroundColor Cyan
Write-Host "   oxide-embed context `"task`"    # Generate token-budgeted prompt context" -ForegroundColor Cyan
Write-Host "   oxide-embed mcp              # Run stdio MCP server for agent IDEs" -ForegroundColor Cyan
Write-Host "   oxide-embed watch            # Run live incremental re-indexing daemon" -ForegroundColor Cyan
Write-Host "==============================================================================" -ForegroundColor Green
