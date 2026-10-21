@echo off
title BOS Full-Stack Launcher
echo ============================================================
echo        BOS (Business Operating System) Full-Stack Runner
echo ============================================================
echo Starting Backend API Server on http://localhost:3000...
start "BOS Backend API Server" cmd /k "cargo run --bin bosc-cli -- serve --port 3000 --db postgres://postgres:postgres@localhost:5432/bos_dev"

echo Starting Frontend Studio on http://localhost:5173...
cd /d "%~dp0ui"
start "BOS Frontend Studio" cmd /k "npm run dev"

echo.
echo ============================================================
echo Both services launched in separate windows!
echo - Backend API:   http://localhost:3000
echo - Frontend UI:   http://localhost:5173
echo ============================================================
