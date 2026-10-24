# Runs the given test command, prints only failure/summary lines, preserves exit code.
$line = $args -join ' '
$lines = & cmd.exe /d /c "$line 2>&1"
$code = $LASTEXITCODE
$pat = '(^error(\[|:)|^warning: unused|FAILED|panicked|^failures:|^---- |test result:|assertion|left:|right:|npm ERR!|\bnot ok\b|\bFAIL\b|Error:|✗|×)'
$keep = New-Object System.Collections.Generic.List[string]
for ($i = 0; $i -lt $lines.Count; $i++) {
  if ([string]$lines[$i] -match $pat) { $end = [Math]::Min($i + 6, $lines.Count - 1); for ($j = $i; $j -le $end; $j++) { $keep.Add([string]$lines[$j]) }; $i = $end }
}
$keep = $keep | Select-Object -First 200
"[test-filter] $line -> exit $code; showing $(@($keep).Count) of $($lines.Count) lines (BOS_RAW_TEST_OUTPUT=1 for full)"
if (@($keep).Count -eq 0) { $lines | Select-Object -Last 15 } else { $keep }
exit $code