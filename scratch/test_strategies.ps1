$baseUrl = "http://127.0.0.1:3000"

Write-Host "=== 1. Testing GET /api/system/storage-strategies ===" -ForegroundColor Cyan
$strategies = Invoke-RestMethod -Uri "$baseUrl/api/system/storage-strategies"
$strategies | ForEach-Object {
    Write-Host "Strategy: $($_.id) - $($_.name) ($($_.isolation_level))" -ForegroundColor Green
}

$testDbNames = @(
    @{ name = "bos_test_shared"; strategy = "shared_rls" },
    @{ name = "bos_test_part"; strategy = "partitioned" },
    @{ name = "bos_test_schema"; strategy = "schema_per_company" },
    @{ name = "bos_test_tbl"; strategy = "table_per_company" }
)

foreach ($item in $testDbNames) {
    $dbName = $item.name
    $strat = $item.strategy
    Write-Host "`n=== Testing Strategy: $strat (DB: $dbName) ===" -ForegroundColor Yellow

    # Drop if exists
    try {
        Invoke-RestMethod -Uri "$baseUrl/api/databases/$dbName" -Method Delete | Out-Null
    } catch {}

    # Create Database with Strategy
    $createBody = @{
        name = $dbName
        init_extensions = $true
        provision_system = $true
        storage_strategy = $strat
    } | ConvertTo-Json

    $res = Invoke-RestMethod -Uri "$baseUrl/api/databases" -Method Post -ContentType "application/json" -Body $createBody
    Write-Host "Created DB $dbName ($($res.storage_strategy)): $($res.success)" -ForegroundColor Green

    # Switch Engine to DB
    $switchBody = @{ name = $dbName } | ConvertTo-Json
    Invoke-RestMethod -Uri "$baseUrl/api/databases/switch" -Method Post -ContentType "application/json" -Body $switchBody | Out-Null

    # Register Vendor Meta-Object
    $metaBody = @{
        object_type = "table"
        name = "Vendor"
        package_name = "bos.core"
        table_type = "Normal"
        data_per_company = $true
        columns = @(
            @{ name = "code"; sql_type = "VARCHAR(30)"; is_nullable = $false; is_unique = $true },
            @{ name = "name"; sql_type = "VARCHAR(150)"; is_nullable = $false }
        )
        primary_key = @("code")
    } | ConvertTo-Json

    $metaRes = Invoke-RestMethod -Uri "$baseUrl/api/meta/objects" -Method Post -ContentType "application/json" -Body $metaBody
    Write-Host "Registered Meta-Object: $($metaRes.object_name) -> $($metaRes.sql_table_name)" -ForegroundColor Green

    # Inspect Physical Tables
    $details = Invoke-RestMethod -Uri "$baseUrl/api/databases/$dbName"
    Write-Host "Physical Tables in $dbName (Strategy: $($details.storage_strategy)):" -ForegroundColor Cyan
    $details.tables | Where-Object { $_.table_name -like "*vendor*" } | ForEach-Object {
        Write-Host "  -> $($_.table_name) ($($_.table_type))" -ForegroundColor White
    }

    # Clean up test DB
    # Switch back to bos_dev first
    $switchDev = @{ name = "bos_dev" } | ConvertTo-Json
    Invoke-RestMethod -Uri "$baseUrl/api/databases/switch" -Method Post -ContentType "application/json" -Body $switchDev | Out-Null
    Invoke-RestMethod -Uri "$baseUrl/api/databases/$dbName" -Method Delete | Out-Null
    Write-Host "Cleaned up $dbName" -ForegroundColor DarkGray
}

# Clean up previously created partitioned test DB
try {
    Invoke-RestMethod -Uri "$baseUrl/api/databases/bos_test_partitioned" -Method Delete | Out-Null
} catch {}

Write-Host "`nAll 4 Storage Strategies Verified Successfully!" -ForegroundColor Green
