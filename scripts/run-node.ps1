param(
    [string]$Config = "bos-node.toml",
    [switch]$Release,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

# Resolve Config file path searching upwards
$resolvedConfig = $Config
$checkDir = (Get-Location).Path
while (-not (Test-Path $resolvedConfig)) {
    $candidate = Join-Path $checkDir $Config
    if (Test-Path $candidate) {
        $resolvedConfig = $candidate
        Set-Location (Split-Path -Parent $candidate)
        break
    }
    $parent = Split-Path -Parent $checkDir
    if (-not $parent -or $parent -eq $checkDir) { break }
    $checkDir = $parent
}

# Extract port from config (default 3000)
$port = 3000
if (Test-Path $resolvedConfig) {
    $content = Get-Content $resolvedConfig -Raw
    if ($content -match 'port\s*=\s*(\d+)') {
        $port = [int]$matches[1]
    }
}

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "         BOS Server Node Launcher (with Port Guardian)      " -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "Profile Config: $resolvedConfig" -ForegroundColor Gray
Write-Host "Target Port:    $port" -ForegroundColor Gray
Write-Host "============================================================" -ForegroundColor Cyan

# Port Guardian: Inspect and reclaim port if occupied
$connections = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue
if ($connections) {
    $pids = $connections | Select-Object -ExpandProperty OwningProcess -Unique
    foreach ($pidToKill in $pids) {
        if ($pidToKill -eq 0 -or $pidToKill -eq $PID) { continue }
        $proc = Get-Process -Id $pidToKill -ErrorAction SilentlyContinue
        $pName = if ($proc) { $proc.ProcessName } else { "Unknown" }
        Write-Host "⚠️  Port $port is currently occupied by process '$pName' (PID: $pidToKill)." -ForegroundColor Yellow
        Write-Host "🔄 Terminating stale process $pidToKill to prevent 'socket in use' (os error 10048)..." -ForegroundColor Yellow
        Stop-Process -Id $pidToKill -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Milliseconds 1200
}

# Verify port was cleanly freed
$stillInUse = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue
if ($stillInUse) {
    Write-Host "⚠️  Warning: Port $port still has active sockets, waiting 1s..." -ForegroundColor DarkYellow
    Start-Sleep -Seconds 1
}

# Run the node
$cargoArgs = @("run")
if ($Release) {
    $cargoArgs += "--release"
}
$cargoArgs += @("-p", "bosc-cli", "--", "node", "run", "--config", $resolvedConfig)

Write-Host "🚀 Launching BOS Server Node..." -ForegroundColor Green
& cargo @cargoArgs
