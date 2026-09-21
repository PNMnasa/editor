<#
.SYNOPSIS
Keeps a snapshot ref under refs/backup/<name> before destructive git operations.

.DESCRIPTION
Aliases the current commit (or a given commit) as refs/backup/<name>, e.g. for
branch experiments the user may force-delete later. Cheap, recoverable, and does
not touch working tree.

.PARAMETER Name
Snapshot ref name under refs/backup/ (letters, digits, _ . / -).

.PARAMETER Commit
Commit to snapshot. Default: HEAD.

.EXAMPLE
.\scripts\snapshot-ref.ps1 -Name experimental
.EXAMPLE
.\scripts\snapshot-ref.ps1 -Name old-ui -Commit abc1234
#>
param(
    [Parameter(Mandatory)][string]$Name,
    [string]$Commit = "HEAD"
)

$ErrorActionPreference = "Stop"

if ($Name -notmatch '^[a-zA-Z0-9_./-]+$') {
    Write-Error "Invalid snapshot name '$Name' (allowed: letters, digits, _ . / -)"
    exit 1
}

$full = "refs/backup/$Name"
Write-Host "==> Snapshot $full -> $Commit"
git update-ref $full $Commit
if ($LASTEXITCODE -ne 0) {
    Write-Error "git update-ref failed"
    exit 1
}

git rev-parse --short $full
if ($LASTEXITCODE -ne 0) {
    Write-Error "git rev-parse failed"
    exit 1
}