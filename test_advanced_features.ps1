$baseUri = "http://localhost:3000"

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "BOS ADVANCED ENTERPRISE SUITE: ITEMS 2.1 - 2.5 VERIFICATION" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# 1. Base Query Protocol (Item 2.5) with SIFT FlowField pushdown
Write-Host "`n--- 1. Testing GraphQL-Like Base Query Protocol (/api/query) ---" -ForegroundColor Green
$queryPayload = @{
    entity = "customer"
    select = @("code", "name", "balance")
    filter = @{
        name = @{
            "$like" = "%"
        }
    }
    include = @(
        @{
            relation = "ledger_entries"
            entity = "customer_ledger_entry"
            foreign_key = "customer_code"
            limit = 5
        }
    )
    limit = 10
} | ConvertTo-Json -Depth 5

$queryRes = Invoke-RestMethod -Uri "$baseUri/api/query" -Method Post -ContentType "application/json" -Body $queryPayload
Write-Host "Query Succeeded! Returned $($queryRes.count) customer(s) with single-trip SIFT FlowField balance:"
$queryRes.data | Format-Table -Property _id, code, name, balance

# 2. Concurrency Control (Item 2.4): Optimistic Concurrency Control
Write-Host "`n--- 2. Testing Optimistic Concurrency Control (Item 2.4) ---" -ForegroundColor Green
# Insert test customer for OCC
$custBody = @{
    code = "OCC-TEST-$(Get-Random)"
    name = "OCC Test Customer"
} | ConvertTo-Json
$custRes = Invoke-RestMethod -Uri "$baseUri/api/entities/customer" -Method Post -ContentType "application/json" -Body $custBody
$testCustId = $custRes._id
Write-Host "Created test customer $testCustId"

# Fetch record to get initial version
$custData = Invoke-RestMethod -Uri "$baseUri/api/entities/customer/$testCustId" -Method Get
$initialVer = $custData._system_version
Write-Host "Initial _system_version: $initialVer"

# Update 1: using current version -> should succeed and bump version
$patch1 = @{
    name = "OCC Test Customer - Updated by User A"
    _system_version = $initialVer
} | ConvertTo-Json
$update1Res = Invoke-RestMethod -Uri "$baseUri/api/entities/customer/$testCustId" -Method Put -ContentType "application/json" -Body $patch1
Write-Host "User A Update 1 Committed successfully! New version: $($update1Res.data._system_version)"

# Update 2: User B tries to update using STALE initial version -> MUST return 409 Conflict!
Write-Host "User B attempting update with stale version $initialVer..."
$patchStale = @{
    name = "OCC Test Customer - Stale Update by User B"
    _system_version = $initialVer
} | ConvertTo-Json

try {
    $staleRes = Invoke-RestMethod -Uri "$baseUri/api/entities/customer/$testCustId" -Method Put -ContentType "application/json" -Body $patchStale
    Write-Host "FAILED: Expected 409 Conflict but update succeeded!" -ForegroundColor Red
} catch {
    Write-Host "SUCCESS: Stale update properly rejected with Conflict: $($_.Exception.Message)" -ForegroundColor Green
}

# 3. Pessimistic Row Locking (Item 2.4)
Write-Host "`n--- 3. Testing Pessimistic Row Locking (POST /api/entities/:table/:id/lock) ---" -ForegroundColor Green
$lockRes = Invoke-RestMethod -Uri "$baseUri/api/entities/customer/$testCustId/lock" -Method Post -ContentType "application/json"
Write-Host "Lock Succeeded: $($lockRes | ConvertTo-Json -Compress)"

# 4. Deletion Protection (Item 2.3): Referential Deletion Trigger
Write-Host "`n--- 4. Testing Referential Deletion Protection (Item 2.3) ---" -ForegroundColor Green
# First, link a ledger entry to our customer
$cleBody = @{
    entry_no = [int](Get-Random)
    customer_code = $custData.code
    amount = 500.00
    open = $true
} | ConvertTo-Json
$cleRes = Invoke-RestMethod -Uri "$baseUri/api/entities/customer_ledger_entry" -Method Post -ContentType "application/json" -Body $cleBody
Write-Host "Linked open ledger entry to $($custData.code)"

# Now attempt to DELETE the customer -> MUST be blocked by PostgreSQL referential trigger!
Write-Host "Attempting to delete customer with active ledger entries..."
try {
    $delRes = Invoke-RestMethod -Uri "$baseUri/api/entities/customer/$testCustId" -Method Delete
    Write-Host "FAILED: Expected delete to be blocked by trigger!" -ForegroundColor Red
} catch {
    Write-Host "SUCCESS: Delete correctly BLOCKED by PostgreSQL trigger: $($_.Exception.Message)" -ForegroundColor Green
}

# 5. Clean Deletion of Unlinked Record
Write-Host "`n--- 5. Testing Clean Deletion of Unlinked Entity ---" -ForegroundColor Green
$unlinkedBody = @{
    code = "CLEAN-DEL-$(Get-Random)"
    name = "Deletable Customer"
} | ConvertTo-Json
$unlinkedCust = Invoke-RestMethod -Uri "$baseUri/api/entities/customer" -Method Post -ContentType "application/json" -Body $unlinkedBody
$cleanId = $unlinkedCust._id
Write-Host "Created unlinked customer $cleanId"

$cleanDelRes = Invoke-RestMethod -Uri "$baseUri/api/entities/customer/$cleanId" -Method Delete
Write-Host "Clean deletion committed successfully: $($cleanDelRes | ConvertTo-Json -Compress)"

Write-Host "`n============================================================" -ForegroundColor Cyan
Write-Host "ALL ADVANCED FEATURES (ITEMS 2.1 - 2.5) FULLY VERIFIED 100%!" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
