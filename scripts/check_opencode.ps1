<#
.SYNOPSIS
Validates the allowlist invariants of the opencode bash permissions.

.DESCRIPTION
Checks opencode.json in the repo root:
  1. JSON parses.
  2. permission.bash exists and every value is allow/ask/deny.
  3. The catch-all "*" is the FIRST key (last matching rule wins) and is NOT "allow".
  4. At least one "allow" rule exists.

Run in CI so a config edit cannot silently break the agent loop.

.EXAMPLE
./scripts/check_opencode.ps1
#>
$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "..\opencode.json"

if (-not (Test-Path -LiteralPath $path)) {
    Write-Error "opencode.json not found at $path"
    exit 1
}

try {
    $cfg = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
} catch {
    Write-Error "opencode.json is not valid JSON: $($_.Exception.Message)"
    exit 1
}

$bash = $cfg.permission.bash
if ($null -eq $bash) {
    Write-Error "permission.bash missing in opencode.json"
    exit 1
}

$names = @($bash.PSObject.Properties.Name)
if ($names.Count -eq 0) {
    Write-Error "permission.bash is empty"
    exit 1
}

if ($names[0] -ne "*") {
    Write-Error "Rule order broken: catch-all '*' must be the FIRST key (last matching rule wins). First key is '$($names[0])'"
    exit 1
}

$catchall = "$($bash.'*')"
if ($catchall -eq "allow") {
    Write-Error "Catch-all '*' must not be 'allow' (would auto-run every command)"
    exit 1
}

$allowCount = 0
foreach ($p in $bash.PSObject.Properties) {
    $value = "$($p.Value)"
    if ($value -notin @("allow", "ask", "deny")) {
        Write-Error "Rule '$($p.Name)' has unknown action '$value' (expected allow|ask|deny)"
        exit 1
    }
    if ($value -eq "allow") { $allowCount++ }
}

if ($allowCount -eq 0) {
    Write-Error "permission.bash has no 'allow' rules (allowlist is empty)"
    exit 1
}

Write-Host "opencode.json OK: catch-all '*'=$catchall, $allowCount allow rules, $($names.Count) rules total."