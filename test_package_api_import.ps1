$ErrorActionPreference = "Stop"

Write-Host "============================================================"
Write-Host "VERIFYING PACKAGE IMPORT INTO SELECTED DB VIA REST API"
Write-Host "============================================================"

$baseUri = "http://localhost:3000"

# 1. Health check
$health = Invoke-RestMethod -Uri "$baseUri/api/health" -Method Get
Write-Host "API Server Status:" ($health | ConvertTo-Json -Compress)

# 2. Read the compiled package bundle JSON
$bundleJson = Get-Content -Raw "packages/bos.auth.bospkg"
$bundleBytes = $bundleJson.Length
Write-Host "Loaded package bundle 'packages/bos.auth.bospkg' ($bundleBytes bytes)"

# 3. Create a fresh unprovisioned database
$targetDb = "test_pkg_api_db"
Write-Host "`n--- 1. Creating target database '$targetDb' ---"
$createBody = @{
    name = $targetDb
    init_extensions = $true
    provision_system = $false
} | ConvertTo-Json
$createRes = Invoke-RestMethod -Uri "$baseUri/api/databases" -Method Post -ContentType "application/json" -Body $createBody
Write-Host "Database Creation Response:" ($createRes | ConvertTo-Json -Compress)

# 4. Import package bundle via Dedicated Route: POST /api/databases/{name}/packages/install
Write-Host "`n--- 2. Importing package bundle via POST /api/databases/$targetDb/packages/install ---"
$importRes1 = Invoke-RestMethod -Uri "$baseUri/api/databases/$targetDb/packages/install" -Method Post -ContentType "application/json" -Body $bundleJson
Write-Host "Package Import Result (Dedicated Route):" ($importRes1 | ConvertTo-Json -Compress)

if ($importRes1.package_id -eq "bos.auth" -and $importRes1.status -eq "active") {
    $pkgId = $importRes1.package_id
    $pkgVer = $importRes1.version
    $objs = $importRes1.objects_installed
    Write-Host "SUCCESS: Package '$pkgId' v$pkgVer installed ($objs objects)" -ForegroundColor Green
} else {
    throw "Package installation failed on dedicated route"
}

# 5. Create second fresh database for Query Parameter test: POST /api/packages/install?db=...
$targetDb2 = "test_pkg_query_db"
Write-Host "`n--- 3. Creating second database '$targetDb2' ---"
$createBody2 = @{
    name = $targetDb2
    init_extensions = $true
    provision_system = $false
} | ConvertTo-Json
$createRes2 = Invoke-RestMethod -Uri "$baseUri/api/databases" -Method Post -ContentType "application/json" -Body $createBody2
Write-Host "Database Creation Response:" ($createRes2 | ConvertTo-Json -Compress)

Write-Host "`n--- 4. Importing package bundle via POST /api/packages/install?db=$targetDb2 ---"
$importRes2 = Invoke-RestMethod -Uri "$baseUri/api/packages/install?db=$targetDb2" -Method Post -ContentType "application/json" -Body $bundleJson
Write-Host "Package Import Result (Query Param):" ($importRes2 | ConvertTo-Json -Compress)

if ($importRes2.package_id -eq "bos.auth" -and $importRes2.status -eq "active") {
    $pkgId2 = $importRes2.package_id
    $pkgVer2 = $importRes2.version
    $objs2 = $importRes2.objects_installed
    Write-Host "SUCCESS: Package '$pkgId2' v$pkgVer2 installed ($objs2 objects)" -ForegroundColor Green
} else {
    throw "Package installation failed on query param route"
}

# 6. Cleanup test databases
Write-Host "`n--- 5. Cleaning up test databases ---"
$null = Invoke-RestMethod -Uri "$baseUri/api/databases/${targetDb}?force=true" -Method Delete
$null = Invoke-RestMethod -Uri "$baseUri/api/databases/${targetDb2}?force=true" -Method Delete
Write-Host "Cleaned up $targetDb and $targetDb2" -ForegroundColor Green

Write-Host "`n============================================================"
Write-Host "ALL REST API PACKAGE IMPORT TESTS PASSED SUCCESSFULLY"
Write-Host "============================================================"
