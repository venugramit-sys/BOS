<#
.SYNOPSIS
    Compiles and packages the entire BOS enterprise stack for zero-dependency distribution.
.DESCRIPTION
    1. Compiles the Svelte 5 Enterprise UI Studio into minified static assets (`ui/dist`).
    2. Builds production-optimized Rust binaries (`bosc.exe`).
    3. Assembles a standalone distribution directory (`dist/bos-server/`).
    4. Generates multi-node configuration profiles (All-In-One, NAS Worker).
    5. Includes 1-click launchers (`start.bat`, `start.ps1`, `start.sh`) and Quickstart guide.
    6. Produces a compressed distribution archive (`.zip`) with SHA-256 checksum.
.EXAMPLE
    pwsh ./scripts/build-distribution.ps1
    pwsh ./scripts/build-distribution.ps1 -SkipFrontend
#>

[CmdletBinding()]
param (
    [string]$Version = "0.1.0",
    [string]$OutputDir = "dist/bos-server",
    [switch]$SkipFrontend = $false,
    [switch]$SkipBackend = $false,
    [switch]$NoZip = $false
)

$ErrorActionPreference = "Stop"

$RootDir = Resolve-Path (Join-Path $PSScriptRoot "..")
$DistPath = Join-Path $RootDir $OutputDir
$ZipPath = "$DistPath-v$Version-windows-x64.zip"

Write-Host "==============================================================" -ForegroundColor Cyan
Write-Host " [BOS] ENTERPRISE DISTRIBUTION BUILD PIPELINE" -ForegroundColor Cyan
Write-Host " Target Version : $Version" -ForegroundColor Gray
Write-Host " Root Directory : $RootDir" -ForegroundColor Gray
Write-Host " Output Target  : $DistPath" -ForegroundColor Gray
Write-Host "==============================================================" -ForegroundColor Cyan

# 1. Verification of Required Tooling
Write-Host "`n[1/6] Verifying toolchain environment..." -ForegroundColor Yellow

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo (Rust toolchain) is not found in PATH. Please install Rust."
}
$CargoVer = (cargo --version)
Write-Host "  OK: Rust Toolchain: $CargoVer" -ForegroundColor Green

if (-not $SkipFrontend) {
    if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
        throw "Node.js is not found in PATH. Required to build Svelte 5 frontend."
    }
    if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
        throw "npm is not found in PATH."
    }
    $NodeVer = (node --version)
    Write-Host "  OK: Node.js Runtime: $NodeVer" -ForegroundColor Green
}

# 2. Compile Svelte 5 Enterprise Frontend (`ui/dist`)
$UiDir = Join-Path $RootDir "ui"
$UiDist = Join-Path $UiDir "dist"

if ($SkipFrontend) {
    Write-Host "`n[2/6] Skipping frontend compilation (SkipFrontend specified)..." -ForegroundColor DarkGray
    if (-not (Test-Path $UiDist)) {
        throw "Frontend dist directory does not exist at '$UiDist'. Build it without -SkipFrontend first."
    }
} else {
    Write-Host "`n[2/6] Building Svelte 5 Enterprise UI Studio..." -ForegroundColor Yellow
    Push-Location $UiDir
    try {
        if (-not (Test-Path "node_modules")) {
            Write-Host "  Installing frontend dependencies..." -ForegroundColor Gray
            npm install --silent
        }
        Write-Host "  Executing 'npm run build'..." -ForegroundColor Gray
        npm run build
        if ($LASTEXITCODE -ne 0) {
            throw "Frontend build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
    Write-Host "  OK: Svelte 5 UI Studio compiled successfully into '$UiDist'." -ForegroundColor Green
}

# 3. Compile Production Rust Binaries (`bosc`)
if ($SkipBackend) {
    Write-Host "`n[3/6] Skipping Rust compilation (SkipBackend specified)..." -ForegroundColor DarkGray
} else {
    Write-Host "`n[3/6] Building release binary for 'bosc-cli' (target/release)..." -ForegroundColor Yellow
    Push-Location $RootDir
    try {
        $buildSuccess = $false
        for ($attempt = 1; $attempt -le 3; $attempt++) {
            Write-Host "  Attempt $attempt/3: Running cargo build --release -p bosc-cli --jobs 2..." -ForegroundColor Gray
            cargo build --release -p bosc-cli --jobs 2
            if ($LASTEXITCODE -eq 0) {
                $buildSuccess = $true
                break
            }
            Write-Host "  Retrying in 2 seconds..." -ForegroundColor DarkYellow
            Start-Sleep -Seconds 2
        }
        if (-not $buildSuccess) {
            throw "Cargo release build failed after 3 attempts."
        }
    } finally {
        Pop-Location
    }
    Write-Host "  OK: Release binary built successfully." -ForegroundColor Green
}

$BinarySource = Join-Path $RootDir "target/release/bosc.exe"
if (-not (Test-Path $BinarySource)) {
    $BinarySource = Join-Path $RootDir "target/release/bosc-cli.exe"
}
if (-not (Test-Path $BinarySource)) {
    $BinarySource = Join-Path $RootDir "target/debug/bosc-cli.exe"
}
if (-not (Test-Path $BinarySource)) {
    throw "Release binary not found at '$BinarySource'."
}

# 4. Assemble Distribution Directory Layout
Write-Host "`n[4/6] Assembling distribution package layout at '$DistPath'..." -ForegroundColor Yellow

if (Test-Path $DistPath) {
    try {
        Remove-Item -Path $DistPath -Recurse -Force -ErrorAction Stop
    } catch {
        Write-Host "  Note: Updating existing distribution directory in-place..." -ForegroundColor DarkYellow
    }
}

$BinDir = Join-Path $DistPath "bin"
$ConfigDir = Join-Path $DistPath "config"
$TargetUiDir = Join-Path $DistPath "ui/dist"

New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
New-Item -ItemType Directory -Path $TargetUiDir -Force | Out-Null

Copy-Item -Path $BinarySource -Destination (Join-Path $BinDir "bosc.exe") -Force
Write-Host "  OK: Copied binary -> bin/bosc.exe" -ForegroundColor Green

Copy-Item -Path "$UiDist/*" -Destination $TargetUiDir -Recurse -Force
Write-Host "  OK: Copied Svelte 5 UI assets -> ui/dist/" -ForegroundColor Green

$SourcePkgDir = Join-Path $RootDir "packages"
$TargetPkgDir = Join-Path $DistPath "packages"
if (Test-Path $SourcePkgDir) {
    New-Item -ItemType Directory -Path $TargetPkgDir -Force | Out-Null
    Copy-Item -Path "$SourcePkgDir/*" -Destination $TargetPkgDir -Recurse -Force
    Write-Host "  OK: Copied BOS packages -> packages/" -ForegroundColor Green
}

# 5. Generate Configuration Profiles & Launchers
Write-Host "`n[5/6] Generating default configuration profiles and launchers..." -ForegroundColor Yellow

$AllInOneToml = @'
# =============================================================================
# BOS Server Node Profile: Primary All-In-One Server
# Serves UI, REST/GraphQL API, and runs background NAS worker
# =============================================================================

[node]
id = "bos-node-prod-01"
name = "BOS Enterprise Primary Node"
role = "all_in_one"

[network]
host = "0.0.0.0"
port = 3000
cors_origins = ["*"]

[ui]
enabled = true
static_dir = "./ui/dist"
index_file = "index.html"

[database]
url = "postgres://postgres:postgres@localhost:5432/bos_dev"
min_pool_size = 5
max_pool_size = 30
storage_strategy = "shared_rls"
connection_timeout_seconds = 10

[defaults]
default_tenant = "DEFAULT_TENANT"
default_company = "MAIN_COMPANY"

[nas_worker]
enabled = true
poll_interval_ms = 1000
batch_size = 50
process_outbox = true
process_job_queue = true
'@
Set-Content -Path (Join-Path $ConfigDir "bos-node.toml") -Value $AllInOneToml -Encoding utf8

$NasToml = @'
# =============================================================================
# BOS Server Node Profile: Dedicated Headless NAS Batch Worker
# Headless daemon running background jobs, GL posting, and outbox distribution
# =============================================================================

[node]
id = "bos-nas-worker-01"
name = "BOS Headless NAS Worker Node"
role = "nas_worker"

[network]
host = "127.0.0.1"
port = 3001
cors_origins = []

[ui]
enabled = false

[database]
url = "postgres://postgres:postgres@localhost:5432/bos_dev"
min_pool_size = 3
max_pool_size = 15
storage_strategy = "shared_rls"
connection_timeout_seconds = 10

[defaults]
default_tenant = "DEFAULT_TENANT"
default_company = "MAIN_COMPANY"

[nas_worker]
enabled = true
poll_interval_ms = 500
batch_size = 100
process_outbox = true
process_job_queue = true
'@
Set-Content -Path (Join-Path $ConfigDir "bos-nas.toml") -Value $NasToml -Encoding utf8

$QuickstartMd = @'
# BOS Enterprise Server - Quickstart & Distribution Guide

Welcome to BOS (Business Operating System).
This standalone package contains the complete 3-Tier Enterprise Server runtime and Svelte 5 UI Studio.

---

## 1. Quick Launch (3-Step Setup)

### Step 1: Configure Database Connection
Open `config/bos-node.toml` and adjust the PostgreSQL connection string:
```toml
[database]
url = "postgres://username:password@localhost:5432/your_database"
```

### Step 2: Validate Configuration
In PowerShell or Command Prompt, run:
```powershell
.\bin\bosc.exe node check --config config/bos-node.toml
```

### Step 3: Start the Server Node
Double-click `start.bat` or run:
```powershell
.\bin\bosc.exe node run --config config/bos-node.toml
```

The server will start listening at:
👉 http://localhost:3000

---

## 2. User Sign-In (Zero DB Knowledge)

When opening http://localhost:3000:
1. Users log in with solely Username and Password.
2. Database connection strings, SQL dialect, and ports remain securely encapsulated on the server node.
3. The server automatically attaches users to the configured company and tenant.

---

## 3. Running Multi-Node Topologies (Like MS Navision NAS)

You can launch multiple independent server nodes from this same installation:

* Interactive Web Server (Port 3000):
  .\bin\bosc.exe node run --config config/bos-node.toml

* Dedicated Headless NAS Worker (Port 3001):
  .\bin\bosc.exe node run --config config/bos-nas.toml
'@
Set-Content -Path (Join-Path $DistPath "QUICKSTART.md") -Value $QuickstartMd -Encoding utf8

$StartBat = @'
@echo off
title BOS Enterprise Server Node
cd /d "%~dp0"
echo ========================================================
echo  Starting BOS Enterprise Server Node...
echo ========================================================
.\bin\bosc.exe node run --config config\bos-node.toml
pause
'@
Set-Content -Path (Join-Path $DistPath "start.bat") -Value $StartBat -Encoding ascii

$StartPs1 = @'
Set-Location -Path $PSScriptRoot
Write-Host "Starting BOS Enterprise Server Node..." -ForegroundColor Cyan
& ".\bin\bosc.exe" node run --config "config/bos-node.toml"
'@
Set-Content -Path (Join-Path $DistPath "start.ps1") -Value $StartPs1 -Encoding utf8

$StartSh = @'
#!/usr/bin/env bash
cd "$(dirname "$0")"
echo "Starting BOS Enterprise Server Node..."
./bin/bosc node run --config config/bos-node.toml
'@
Set-Content -Path (Join-Path $DistPath "start.sh") -Value $StartSh -Encoding utf8

Write-Host "  OK: Generated config profiles (config/bos-node.toml, config/bos-nas.toml)" -ForegroundColor Green
Write-Host "  OK: Generated 1-click launchers (start.bat, start.ps1, start.sh, QUICKSTART.md)" -ForegroundColor Green

# 6. Compress Package Archive (.zip)
if (-not $NoZip) {
    Write-Host "`n[6/6] Creating compressed distribution archive '$ZipPath'..." -ForegroundColor Yellow
    if (Test-Path $ZipPath) {
        Remove-Item -Path $ZipPath -Force
    }

    if (Get-Command tar.exe -ErrorAction SilentlyContinue) {
        & tar.exe -a -c -f $ZipPath -C $DistPath .
    } else {
        Compress-Archive -Path "$DistPath/*" -DestinationPath $ZipPath -CompressionLevel Optimal
    }
    $ZipItem = Get-Item $ZipPath
    $ZipSizeMb = [math]::Round($ZipItem.Length / 1MB, 2)

    $Hash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash
    Set-Content -Path "$ZipPath.sha256" -Value "$Hash  $($ZipItem.Name)"

    Write-Host "  OK: Compressed Archive: $ZipPath ($ZipSizeMb MB)" -ForegroundColor Green
    Write-Host "  OK: SHA-256 Checksum  : $Hash" -ForegroundColor Green
} else {
    Write-Host "`n[6/6] Skipping zip compression (NoZip specified)." -ForegroundColor DarkGray
}

Write-Host "`n==============================================================" -ForegroundColor Cyan
Write-Host " BOS DISTRIBUTION BUILD COMPLETE AND READY FOR DEPLOYMENT!" -ForegroundColor Green
Write-Host " Standalone Folder : $DistPath" -ForegroundColor White
if (-not $NoZip) {
    Write-Host " Standalone Archive: $ZipPath" -ForegroundColor White
}
Write-Host "==============================================================" -ForegroundColor Cyan
