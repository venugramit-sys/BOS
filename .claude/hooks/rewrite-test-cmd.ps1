# PreToolUse hook (Bash/PowerShell tool): wraps plain `cargo test ...` / `npm test` / `npm run test...`
# so only failures + summaries reach the model. Opt out: set BOS_RAW_TEST_OUTPUT=1. Undo: remove "hooks" from .claude/settings.json.
$ErrorActionPreference = 'Stop'
try { $in = [Console]::In.ReadToEnd() | ConvertFrom-Json } catch { exit 0 }
if ($env:BOS_RAW_TEST_OUTPUT -eq '1') { exit 0 }
$cmd = [string]$in.tool_input.command
if (-not $cmd) { exit 0 }
# only simple single commands: no pipes, chaining, redirects or quotes
if ($cmd -notmatch '^\s*(cargo\s+test|npm\s+test|npm\s+run\s+test[\w:-]*)(\s+[\w@./:=,+-]+)*\s*$') { exit 0 }
$filter = Join-Path $PSScriptRoot 'test-filter.ps1'
$new = 'powershell -NoProfile -ExecutionPolicy Bypass -File "' + $filter.Replace('\','/') + '" ' + $cmd.Trim()
$out = @{ hookSpecificOutput = @{ hookEventName = 'PreToolUse'; permissionDecision = 'allow'
  permissionDecisionReason = 'test output filtered (BOS_RAW_TEST_OUTPUT=1 for raw)'
  updatedInput = @{ command = $new } } }
if ($in.tool_input.description) { $out.hookSpecificOutput.updatedInput.description = $in.tool_input.description }
$out | ConvertTo-Json -Depth 5 -Compress
exit 0