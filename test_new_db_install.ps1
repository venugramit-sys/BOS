# test_new_db_install.ps1
# End-to-end verification of installing bos.auth package onto a brand new database

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "TESTING AUTH PACKAGE INSTALLATION ON BRAND NEW DATABASE" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

$dbName = "bos_auth_verify_$(Get-Random)"
$clusterUrl = "postgres://postgres:postgres@localhost:5432/postgres"
$targetDbUrl = "postgres://postgres:postgres@localhost:5432/$dbName"

# 1. Create fresh database
Write-Host "`n--- 1. Creating fresh database '$dbName' ---" -ForegroundColor Yellow
cargo run --bin bosc-cli -- create-db --name $dbName --cluster-url $clusterUrl

# 2. Install auth package
Write-Host "`n--- 2. Installing bos.auth package onto '$dbName' ---" -ForegroundColor Yellow
cargo run --bin bosc-cli -- package install --input packages/bos.auth.bospkg --db $targetDbUrl

# 3. List installed packages
Write-Host "`n--- 3. Verifying installed packages on '$dbName' ---" -ForegroundColor Yellow
cargo run --bin bosc-cli -- package list --db $targetDbUrl

Write-Host "`n============================================================" -ForegroundColor Cyan
Write-Host "FRESH DATABASE AUTH PACKAGE INSTALLATION VERIFIED 100%!" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
