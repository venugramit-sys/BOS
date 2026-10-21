param(
    [string]$Db = "postgres://postgres:postgres@localhost:5432/postgres",
    [int]$BackendPort = 3000,
    [switch]$SeparateWindows
)

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "       BOS (Business Operating System) Full-Stack Runner    " -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "Database:     $Db" -ForegroundColor Gray
Write-Host "Backend API:  http://localhost:$BackendPort" -ForegroundColor Gray
Write-Host "Frontend UI:  http://localhost:5173" -ForegroundColor Gray
Write-Host "============================================================" -ForegroundColor Cyan

if ($SeparateWindows) {
    Write-Host "`nSpawning Backend & Frontend in separate terminal windows..." -ForegroundColor Yellow
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "cargo run --bin bosc-cli -- serve --port $BackendPort --db $Db"
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "Set-Location ui; npm run dev"
    Write-Host "Both services launched successfully in separate windows!" -ForegroundColor Green
    return
}

Write-Host "`n[1/2] Starting Backend API Server..." -ForegroundColor Green
$backendProc = Start-Process -FilePath "cargo" -ArgumentList "run", "--bin", "bosc-cli", "--", "serve", "--port", "$BackendPort", "--db", "$Db" -PassThru

Write-Host "[2/2] Starting Frontend Vite Dev Server..." -ForegroundColor Cyan
Write-Host "`nPress Ctrl+C to terminate both servers.`n" -ForegroundColor DarkGray

try {
    Push-Location ui
    npm run dev
}
finally {
    Pop-Location
    if ($backendProc -and -not $backendProc.HasExited) {
        Write-Host "`nStopping Backend API Server (PID: $($backendProc.Id))..." -ForegroundColor Yellow
        Stop-Process -Id $backendProc.Id -Force -ErrorAction SilentlyContinue
    }
    Write-Host "All BOS services stopped cleanly." -ForegroundColor Gray
}
