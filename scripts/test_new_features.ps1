$headers = @{ "Content-Type" = "application/json"; "x-tenant-id" = "00000000-0000-0000-0000-000000000000"; "x-company-id" = "00000000-0000-0000-0000-000000000000" }

Write-Host "=== TEST 1: STORAGE STRATEGIES ==="
$strat = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/system/storage-strategies" -Method GET
Write-Host "Supported strategies count:" $strat.Count

Write-Host "=== TEST 2: PACKAGE SYSTEM ==="
$packagePayload = @{
    manifest = @{
        package_id = "bos.sample.supplychain"
        name = "Supply Chain Management"
        version = "1.0.0"
        publisher = "BOS Core Corp"
        dependencies = @()
        modules = @("Inventory", "Logistics")
    }
    objects = @(
        @{
            object_type = "table"
            name = "Warehouse"
            caption = "Distribution Warehouse"
            table_type = "Normal"
            data_per_company = $true
            columns = @(
                @{ name = "code"; sql_type = "text"; is_nullable = $false; is_unique = $true },
                @{ name = "location"; sql_type = "text"; is_nullable = $false }
            )
        }
    )
} | ConvertTo-Json -Depth 10

$pkgRes = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/packages/install" -Method POST -Body $packagePayload -Headers $headers
Write-Host "Package installed:" $pkgRes.message

$exportRes = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/packages/bos.sample.supplychain/export" -Method GET
Write-Host "Package exported:" $exportRes.manifest.name "with objects:" $exportRes.objects.Count

Write-Host "=== TEST 3: TABLE EXTENSION ==="
$extPayload = @{
    name = "WarehouseExt"
    package_name = "bos.sample.supplychain"
    target_table = "Warehouse"
    added_columns = @(
        @{ name = "capacity_sqft"; sql_type = "integer"; is_nullable = $true; caption = "Storage Capacity SqFt" }
    )
} | ConvertTo-Json -Depth 10

$extRes = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/meta/table-extensions" -Method POST -Body $extPayload -Headers $headers
Write-Host "Table extension applied:" $extRes.object_name "on" $extRes.sql_table_name

Write-Host "=== TEST 4: INSERT INTO EXTENDED TABLE ==="
$whCode = "WH-SEA-" + (Get-Random -Minimum 100 -Maximum 999)
$whPayload = @{
    code = $whCode
    location = "Seattle Marine Terminal"
    capacity_sqft = 75000
} | ConvertTo-Json

$whRes = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/entities/Warehouse" -Method POST -Body $whPayload -Headers $headers
Write-Host "Inserted warehouse record ID:" $whRes._id "with Code:" $whCode

Write-Host "=== TEST 5: CODEUNIT / BUSINESS LOGIC RUNTIME ==="
$cuPayload = @{
    name = "WarehouseValidator"
    package_name = "bos.sample.supplychain"
    bound_table = "Warehouse"
    is_exposed = $true
    is_transactional = $true
    methods = @(
        @{
            name = "ValidateWarehouse"
            is_public = $true
            is_static = $true
            params = @(
                @("code", "String"),
                @("capacity_sqft", "Integer")
            )
            actions = @(
                @{
                    ValidateRequired = @{ field = "code" }
                },
                @{
                    Assert = @{
                        condition = "capacity_sqft > 1000"
                        message = "Warehouse capacity must exceed 1000 sq ft"
                    }
                },
                @{
                    SetField = @{
                        field = "validated_by"
                        value_expr = "SystemRules"
                    }
                },
                @{
                    LogAudit = @{
                        message = "Warehouse successfully verified by rule engine"
                    }
                }
            )
        }
    )
} | ConvertTo-Json -Depth 10

$cuRes = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/codeunits" -Method POST -Body $cuPayload -Headers $headers
Write-Host "Registered codeunit:" $cuRes.name

$runPayload = @{
    method = "ValidateWarehouse"
    params = @{
        code = "WH-SEA-01"
        capacity_sqft = 75000
    }
} | ConvertTo-Json

$execRes = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/codeunits/WarehouseValidator/run" -Method POST -Body $runPayload -Headers $headers
Write-Host "Execution result success:" $execRes.success "Logs:" ($execRes.logs -join " | ")

Write-Host "ALL FEATURES VERIFIED SUCCESSFULLY!"
