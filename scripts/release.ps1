<#
.SYNOPSIS
Quick release pipeline: commit, push develop, merge into main, tag, back to develop.

.DESCRIPTION
One command that:
  1. Runs the 4 CI steps (fmt, clippy, test, release build) - can be skipped.
  2. Commits all pending changes with a clear message (skips if nothing to commit).
  3. Pushes develop to origin.
  4. Fast-forwards main to develop and pushes main.
  5. Creates a tag (auto patch bump from the latest tag when -Version is omitted) and pushes it.
  6. Returns to develop locally.

.PARAMETER Message
Commit message. Default: "chore: auto commit for release".

.PARAMETER Version
Tag name, e.g. "v0.2.0". Default: auto patch bump from the latest tag.

.PARAMETER SkipChecks
Skip the 4 CI steps.

.EXAMPLE
.\scripts\release.ps1 "feat: add tricky feature"

.EXAMPLE
.\scripts\release.ps1 "fix: typo" -Version v0.2.1 -SkipChecks
#>
param(
    [string]$Message = "",
    [string]$Version = "",
    [switch]$SkipChecks
)

$ErrorActionPreference = "Stop"

function Fail([string]$Msg) {
    Write-Error $Msg
    exit 1
}

function Check-LastExit($Label) {
    if ($LASTEXITCODE -ne 0) { Fail "$Label failed (exit $LASTEXITCODE)" }
}

# 1. Branch guard
$current = git branch --show-current
Check-LastExit "git branch --show-current"
if ($current -ne "develop") {
    Fail "Must be on branch 'develop' (currently: '$current')"
}

# 2. CI checks
if (-not $SkipChecks) {
    Write-Host "==> Running CI checks..."
    Write-Host "==> cargo fmt --check"
    cargo fmt --check
    Check-LastExit "cargo fmt --check"
    Write-Host "==> cargo clippy --all-targets -- -D warnings"
    cargo clippy --all-targets -- -D warnings
    Check-LastExit "cargo clippy"
    Write-Host "==> cargo test"
    cargo test
    Check-LastExit "cargo test"
    Write-Host "==> cargo build --release"
    cargo build --release
    Check-LastExit "cargo build --release"
}

# 3. Commit pending changes
$status = git status --porcelain
Check-LastExit "git status"
if ($status) {
    if (-not $Message) { $Message = "chore: auto commit for release" }
    Write-Host "==> Committing: $Message"
    git add -A
    Check-LastExit "git add -A"
    git commit -m $Message
    Check-LastExit "git commit"
} else {
    Write-Host "==> No changes to commit."
}

# 4. Push develop
Write-Host "==> git push origin develop"
git push origin develop
Check-LastExit "git push origin develop"

# 5. Fast-forward main to develop and push
Write-Host "==> git checkout main"
git checkout main
Check-LastExit "git checkout main"
Write-Host "==> git pull --ff-only origin main"
git pull --ff-only origin main
Check-LastExit "git pull --ff-only origin main"
Write-Host "==> git merge --ff-only develop"
git merge --ff-only develop
Check-LastExit "git merge develop"
Write-Host "==> git push origin main"
git push origin main
Check-LastExit "git push origin main"

# 6. Determine version and tag
if (-not $Version) {
    $last = git describe --tags --abbrev=0 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $last) { $last = "v0.0.0" }
    $parts = ($last -replace "^v", "") -split "\."
    while ($parts.Count -lt 3) { $parts += "0" }
    try {
        $Version = "v$($parts[0]).$($parts[1]).$([int]$parts[2] + 1)"
    } catch {
        Fail "Could not bump version from tag '$last'."
    }
}
Write-Host "==> Tagging: $Version"
git tag $Version
Check-LastExit "git tag"
Write-Host "==> git push origin $Version"
git push origin $Version
Check-LastExit "git push tag"

# 7. Back to develop
Write-Host "==> git checkout develop"
git checkout develop
Check-LastExit "git checkout develop"

Write-Host ""
Write-Host "Done. Released $Version on main; back on develop."