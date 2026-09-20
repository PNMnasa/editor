<#
.SYNOPSIS
Quick release: commit pending changes, push main, create and push a tag.

.DESCRIPTION
Runs the release steps in one command:
  1. Stages and commits all pending changes with the given message.
  2. Pushes main to origin.
  3. Creates a tag from the current HEAD and pushes it (triggers the release workflow).

.PARAMETER Message
Commit message, e.g. "feat: add tricky feature".

.PARAMETER Version
Tag to create, e.g. "v0.2.0".

.EXAMPLE
.\scripts\release.ps1 "feat: add tricky feature" v0.2.0
#>
param(
    [string]$Message,
    [string]$Version
)

$ErrorActionPreference = "Stop"

if (-not $Message) {
    Write-Error "A commit message is required."
    exit 1
}
if (-not $Version) {
    Write-Error "A version tag is required (example: v0.2.0)."
    exit 1
}

git add -A
if ($LASTEXITCODE -ne 0) { exit 1 }
git commit -m $Message
if ($LASTEXITCODE -ne 0) { exit 1 }

git push origin main
if ($LASTEXITCODE -ne 0) { exit 1 }

git tag $Version
if ($LASTEXITCODE -ne 0) { exit 1 }
git push origin $Version
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host ""
Write-Host "Done. Released $Version on main."