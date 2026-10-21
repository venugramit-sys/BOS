<#
.SYNOPSIS
    Unified Master Runner for the BOS QA Tester Agent ('tester' / 'qa-tester').
.DESCRIPTION
    Executes the 4 operational testing pillars:
      1. Feature & Module Task Decomposition Matrix
      2. Code Change & Blast-Radius Detection
      3. Exhaustive API Boundary & Input Fuzzing Suite
      4. Visual Browser UI & UX Verification (Playwright + Headless Chrome/Edge)
.PARAMETER Mode
    Testing mode to execute: 'All', 'Matrix', 'Impact', 'ApiFuzz', or 'Visual'.
    Defaults to 'All'.
.PARAMETER ApiUrl
    Backend API server URL. Defaults to 'http://localhost:3000'.
.PARAMETER UiUrl
    Frontend Svelte 5 studio URL. Defaults to 'http://localhost:5173'.
#>

param(
    [ValidateSet('All', 'Matrix', 'Impact', 'ApiFuzz', 'Visual')]
    [string]$Mode = 'All',
    [string]$ApiUrl = 'http://localhost:3000',
    [string]$UiUrl = 'http://localhost:5173'
)

$ErrorActionPreference = 'Stop'
$ScriptDir = $PSScriptRoot
$RootDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  BOS AUTONOMOUS QA TESTER AGENT EXECUTION SUITE" -ForegroundColor Cyan
Write-Host "  Mode: $Mode | API: $ApiUrl | UI: $UiUrl" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

$env:BOS_API_URL = $ApiUrl
$env:BOS_UI_URL = $UiUrl

# Mode 1: Task Decomposition Matrix
if ($Mode -in @('All', 'Matrix')) {
    Write-Host "`n[Pillar 1/4] Running Feature & Task Decomposition Matrix..." -ForegroundColor Green
    node "$ScriptDir\decompose_tasks.mjs"
}

# Mode 2: Code Change & Blast-Radius Tracker
if ($Mode -in @('All', 'Impact')) {
    Write-Host "`n[Pillar 2/4] Running Code Change & Blast-Radius Tracker..." -ForegroundColor Green
    node "$ScriptDir\track_changes.mjs"
}

# Mode 3: Exhaustive API Fuzzing Suite
if ($Mode -in @('All', 'ApiFuzz')) {
    Write-Host "`n[Pillar 3/4] Running Exhaustive API Boundary & Fuzzing Suite..." -ForegroundColor Green
    node "$ScriptDir\fuzz_api_endpoints.mjs"
}

# Mode 4: Visual Browser UI/UX Verification
if ($Mode -in @('All', 'Visual')) {
    Write-Host "`n[Pillar 4/4] Running Visual Browser UI & UX Verification Suite..." -ForegroundColor Green
    node "$ScriptDir\visual_browser_test.mjs"
}

Write-Host "`n============================================================" -ForegroundColor Cyan
Write-Host "  ALL REQUESTED QA SUITES EXECUTED SUCCESSFULLY" -ForegroundColor Cyan
Write-Host "  Reports & Artifacts stored in scratch/ and docs/" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
