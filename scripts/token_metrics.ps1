# Reproducible token-proxy metrics for BOS. Usage: pwsh -File scripts/token_metrics.ps1
# Approx tokens = bytes / 4 (rough heuristic for English/code; not a tokenizer).
$ErrorActionPreference = 'Stop'
Set-Location (git rev-parse --show-toplevel)
function Sz($p){ if(Test-Path $p){ (Get-Item $p).Length } else { 0 } }
$always = @('AGENTS.md','CLAUDE.md') + (Get-ChildItem .cursor/rules/*.mdc -EA SilentlyContinue | ? { (Get-Content $_ -Raw) -match '(?m)^alwaysApply:\s*true' } | % { Resolve-Path -Relative $_ }) + (Get-ChildItem .agents/rules/*.md -EA SilentlyContinue | ? { (Get-Content $_ -Raw) -match '(?m)^trigger:\s*always_on' } | % { Resolve-Path -Relative $_ })
$ab = ($always | % { Sz $_ } | Measure -Sum).Sum
"Always-loaded files at repo root session start:"; $always | % { "  $_ : $(Sz $_) bytes" }
"Always-loaded total: $ab bytes (~$([int]($ab/4)) tokens est.)"
$nested = Get-ChildItem crates/*/AGENTS.md, ui/AGENTS.md -EA SilentlyContinue
if($nested){ $nb=($nested|Measure Length -Sum).Sum; "Directory-scoped AGENTS.md (loaded only when working in that folder): $($nested.Count) files, $nb bytes (~$([int]($nb/4)) tokens est.)" }
$tracked = @(git ls-files -c)
"Git tracked files: $($tracked.Count)"
$ign = if(Test-Path .cursorignore){ @(git ls-files -c -i --exclude-from=.cursorignore) } else { @() }
$untr = if(Test-Path .cursorignore){ @(git -c core.excludesFile=.cursorignore ls-files -o --exclude-standard) } else { @(git ls-files -o --exclude-standard) }
$idx = @($tracked | ? { $ign -notcontains $_ }) + $untr
$ib = ($idx | ? { Test-Path -LiteralPath $_ -PathType Leaf } | % { (Get-Item -LiteralPath $_).Length } | Measure -Sum).Sum
"Indexable files after .gitignore + .cursorignore: $($idx.Count), $ib bytes (~$([int]($ib/4)) tokens est.)"
$c = git count-objects -vH; "Git object store:"; $c | ? { $_ -match 'size-pack|count' } | % { "  $_" }
