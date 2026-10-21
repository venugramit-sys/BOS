<#
.SYNOPSIS
    Deep Code Quality & Anti-Mock Auditor for the BOS Platform.
    Used by the 'detector' agent to identify fake, mock, simulating, hard-coded, half-baked, or incomplete code paths.
.PARAMETER Target
    Optional path to scan. Defaults to scanning 'crates' and 'ui/src'.
.PARAMETER IncludeTests
    If true, includes test files and modules. Defaults to false (focuses on production paths).
#>
param(
    [string]$Target = "",
    [switch]$IncludeTests = $false,
    [switch]$JsonOutput = $false
)

$rootDir = Split-Path -Parent $PSScriptRoot
if (-not $Target) {
    $scanTargets = @(
        (Join-Path $rootDir "crates"),
        (Join-Path $rootDir "ui\src")
    )
} else {
    $scanTargets = @((Resolve-Path $Target).Path)
}

$patterns = @(
    @{ Pattern = '\btodo!\s*\('; Category = 'HALF_BAKED'; Severity = 'CRITICAL'; Message = 'todo!() macro found in code path' },
    @{ Pattern = '\bunimplemented!\s*\('; Category = 'HALF_BAKED'; Severity = 'CRITICAL'; Message = 'unimplemented!() macro found in code path' },
    @{ Pattern = '\b(mock|fake|simulate|stub|dummy)\b'; Category = 'MOCK_OR_SIMULATED'; Severity = 'HIGH'; Message = 'Potential mock, fake, or simulation keyword detected' },
    @{ Pattern = '\b(TODO|FIXME|WIP|HACK)\b'; Category = 'INCOMPLETE_WORK'; Severity = 'MEDIUM'; Message = 'Unresolved TODO/FIXME marker detected' },
    @{ Pattern = '00000000-0000-0000-0000-000000000000|11111111-1111-1111-1111-111111111111'; Category = 'HARDCODED'; Severity = 'HIGH'; Message = 'Hard-coded dummy UUID detected' },
    @{ Pattern = 'password\s*[:=]\s*["''][^"'']+["'']|secret\s*[:=]\s*["''][^"'']+["'']'; Category = 'HARDCODED'; Severity = 'CRITICAL'; Message = 'Potential hard-coded credential or secret detected' },
    @{ Pattern = '\.unwrap\(\)'; Category = 'UNSAFE_ERROR_HANDLING'; Severity = 'LOW'; Message = 'bare .unwrap() call detected; consider explicit error handling' },
    @{ Pattern = 'console\.(log|warn)\('; Category = 'DEBUG_LEAK'; Severity = 'LOW'; Message = 'console.log/warn left in frontend code' },
    @{ Pattern = '^\s*\$:\s+'; Category = 'SVELTE_ANTI_PATTERN'; Severity = 'HIGH'; Message = 'Legacy Svelte $: reactive declaration found; use Svelte 5 runes ($derived, $effect)' }
)

$flaws = [System.Collections.Generic.List[PSObject]]::new()

foreach ($targetPath in $scanTargets) {
    if (-not (Test-Path $targetPath)) { continue }

    $files = Get-ChildItem -Path $targetPath -Recurse -File | Where-Object {
        $ext = $_.Extension
        ($ext -in @('.rs', '.svelte', '.ts', '.js')) -and
        ($_.FullName -notmatch '[\\/]target[\\/]') -and
        ($_.FullName -notmatch '[\\/]node_modules[\\/]') -and
        ($_.FullName -notmatch '[\\/]dist[\\/]') -and
        ($IncludeTests -or ($_.FullName -notmatch '[\\/]tests[\\/]'))
    }

    foreach ($file in $files) {
        $lines = Get-Content -Path $file.FullName -ErrorAction SilentlyContinue
        if ($null -eq $lines) { continue }

        $lineNum = 0
        $inTestModule = $false

        foreach ($line in $lines) {
            $lineNum++
            if (-not $IncludeTests -and $line -match '#\[cfg\(test\)\]') {
                $inTestModule = $true
            }

            if ($inTestModule -and -not $IncludeTests) {
                # In Rust, test module starts at #[cfg(test)]. For strict audit we check if we skip test module
                # Continue if inside test module block
                continue
            }

            foreach ($p in $patterns) {
                if ($line -match $p.Pattern) {
                    # Filter out this script itself or comments explaining audit
                    if ($line -match 'audit_keywords\.json' -or $line -match 'detect_flaws\.ps1') {
                        continue
                    }

                    $relPath = $file.FullName.Replace($rootDir, '').TrimStart('\').TrimStart('/')
                    $flawObj = [PSCustomObject]@{
                        File = $relPath
                        Line = $lineNum
                        Category = $p.Category
                        Severity = $p.Severity
                        Message = $p.Message
                        MatchSnippet = $line.Trim()
                    }
                    $flaws.Add($flawObj)
                }
            }
        }
    }
}

if ($JsonOutput) {
    $flaws | ConvertTo-Json -Depth 4
    return
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " BOS CODE QUALITY & ANTI-MOCK AUDIT REPORT (DETECTOR)" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

if ($flaws.Count -eq 0) {
    Write-Host "`nVERDICT: FLAWLESS (PASSED)" -ForegroundColor Green
    Write-Host "No mocks, fakes, todo!, unimplemented!, or anti-patterns detected." -ForegroundColor Green
} else {
    Write-Host "`nVERDICT: FLAWS DETECTED (FAILED - ACTION REQUIRED)" -ForegroundColor Red
    Write-Host "Total findings: $($flaws.Count)`n" -ForegroundColor Yellow

    $grouped = $flaws | Group-Object Severity
    foreach ($grp in $grouped) {
        $color = switch ($grp.Name) {
            'CRITICAL' { 'Red' }
            'HIGH'     { 'Magenta' }
            'MEDIUM'   { 'Yellow' }
            default    { 'Gray' }
        }
        Write-Host "[$($grp.Name)]: $($grp.Count) occurrence(s)" -ForegroundColor $color
    }

    Write-Host "`nDetailed Findings:" -ForegroundColor Cyan
    foreach ($f in $flaws) {
        $color = switch ($f.Severity) {
            'CRITICAL' { 'Red' }
            'HIGH'     { 'Magenta' }
            'MEDIUM'   { 'Yellow' }
            default    { 'Gray' }
        }
        Write-Host "[$($f.Severity)] $($f.Category): $($f.File):$($f.Line)" -ForegroundColor $color
        Write-Host "  Snippet: $($f.MatchSnippet)" -ForegroundColor DarkGray
        Write-Host "  Issue:   $($f.Message)" -ForegroundColor DarkGray
    }
}
Write-Host "==========================================================`n" -ForegroundColor Cyan
