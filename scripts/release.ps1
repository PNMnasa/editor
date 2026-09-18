<#
.SYNOPSIS
Quick release pipeline: analyze changes, auto-commit (grouped), push develop, merge main, tag, back to develop.

.DESCRIPTION
One command that:
  1. Runs the 4 CI steps (fmt, clippy, test, release build) - can be skipped.
  2. Analyzes the pending changes and commits them with a Conventional Commits message,
     splitting into multiple commits per area (ci / docs / src / tooling / other).
  3. Pushes develop to origin.
  4. Fast-forwards main to develop and pushes main.
  5. Creates a tag (auto patch bump from the latest tag when -Version is omitted) and pushes it.
  6. Returns to develop locally.

.PARAMETER Message
Optional explicit commit message. When set, everything is committed in a single commit instead of
grouped analysis.

.PARAMETER Version
Tag name, e.g. "v0.2.0". Default: auto patch bump from the latest tag.

.PARAMETER SkipChecks
Skip the 4 CI steps.

.PARAMETER DryRun
Print what would happen without touching git or the compiler.

.EXAMPLE
.\scripts\release.ps1 "feat: add tricky feature"

.EXAMPLE
.\scripts\release.ps1 -Version v0.2.1 -SkipChecks

.EXAMPLE
.\scripts\release.ps1 -SkipChecks -DryRun
#>
param(
    [string]$Message = "",
    [string]$Version = "",
    [switch]$SkipChecks,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

function Fail([string]$Msg) {
    Write-Error $Msg
    exit 1
}

function Check-LastExit($Label) {
    if ($LASTEXITCODE -ne 0) { Fail "$Label failed (exit $LASTEXITCODE)" }
}

# ---- Auto commit message analysis -------------------------------------------

function Get-Bucket([string]$Path) {
    if ($Path -match '^\.github/') { return "ci" }
    if ($Path -match '\.md$') { return "docs" }
    if ($Path -match '^src/.*\.rs$') { return "rust" }
    if ($Path -match '^(opencode\.json|\.opencode/|\.vscode/)') { return "tooling" }
    return "other"
}

function Get-FileStatus([string]$Line) {
    if ($Line.Length -lt 3) { return @{ X = "?"; Path = $Line } }
    $x = $Line.Substring(0, 1)
    $y = $Line.Substring(1, 1)
    return @{ X = if ($x -eq " ") { $y } else { $x }; Path = $Line.Substring(3).Trim('"') }
}

function New-BucketMessage {
    param(
        [string]$Type,
        [string[]]$Files,
        [hashtable]$StatusByPath
    )
    $names = ($Files | ForEach-Object { Split-Path $_ -Leaf }) -join ", "
    $added = $false
    $deleted = $false
    foreach ($f in $Files) {
        if ($StatusByPath[$f].X -eq "A") { $added = $true }
        if ($StatusByPath[$f].X -eq "D") { $deleted = $true }
    }
    switch ($Type) {
        "ci" { return "ci: update workflow files" }
        "docs" {
            if ($deleted) { return "docs: remove $names" }
            return "docs: update documentation ($names)"
        }
        "rust" {
            $diff = git diff --cached -U0 -- $Files
            $addedFns = New-Object System.Collections.Generic.List[string]
            foreach ($line in $diff) {
                if ($line -match '^\++\s*(pub\s+)?fn\s+(\w+)') { $addedFns.Add($matches[2]) }
            }
            if ($deleted) { return "chore: remove source module ($names)" }
            if ($addedFns.Count -gt 0) {
                return "feat: add $($addedFns -join ', ') ($names)"
            }
            if ($added) { return "feat: add module ($names)" }
            return "feat: update ($names)"
        }
        "tooling" { return "chore: update tooling files ($names)" }
        default {
            if ($deleted) { return "chore: remove ($names)" }
            return "chore: update ($names)"
        }
    }
}

function Commit-Groups {
    param([string[]]$StatusLines)
    $statusByPath = @{}
    foreach ($line in $StatusLines) {
        $info = Get-FileStatus $line
        $statusByPath[$info.Path] = $info
    }
    $groups = @{}
    foreach ($info in $statusByPath.Values) {
        if (-not $groups.ContainsKey((Get-Bucket $info.Path))) {
            $groups[(Get-Bucket $info.Path)] = New-Object System.Collections.Generic.List[string]
        }
        $groups[(Get-Bucket $info.Path)].Add($info.Path)
    }

    foreach ($type in ($groups.Keys | Sort-Object)) {
        $files = @($groups[$type])
        Write-Host "==> git add $($files -join ' ')"
        if ($DryRun) {
            git add -- $files
            Check-LastExit "git add"
            $msg = New-BucketMessage -Type $type -Files $files -StatusByPath $statusByPath
            git reset -q -- $files
            Write-Host "==> [dryrun] commit ($type): $msg"
        } else {
            git add -- $files
            Check-LastExit "git add"
            $msg = New-BucketMessage -Type $type -Files $files -StatusByPath $statusByPath
            Write-Host "==> commit ($type): $msg"
            git commit -m $msg
            Check-LastExit "git commit ($type)"
        }
    }
}

# ---- Main -------------------------------------------------------------------

# 1. Branch guard
$current = git branch --show-current
Check-LastExit "git branch --show-current"
if ($current -ne "develop") {
    Fail "Must be on branch 'develop' (currently: '$current')"
}

# 2. CI checks
if ($DryRun) {
    Write-Host "==> [dryrun] would run the 4 CI steps"
} elseif (-not $SkipChecks) {
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
} else {
    Write-Host "==> Skipping CI checks."
}

# 3. Commit pending changes
$status = git status --porcelain
Check-LastExit "git status"
if ($status) {
    if ($Message) {
        Write-Host "==> Committing (single): $Message"
        if (-not $DryRun) {
            git add -A
            Check-LastExit "git add -A"
            git commit -m $Message
            Check-LastExit "git commit"
        } else {
            Write-Host "==> [dryrun] git add -A; git commit -m '$Message'"
        }
    } else {
        Commit-Groups $status
    }
} else {
    Write-Host "==> No changes to commit."
}

# 4. Push develop
Write-Host "==> git push origin develop"
if (-not $DryRun) {
    git push origin develop
    Check-LastExit "git push origin develop"
}

# 5. Fast-forward main to develop and push
Write-Host "==> git checkout main; git pull --ff-only origin main; git merge --ff-only develop; git push origin main"
if (-not $DryRun) {
    git checkout main
    Check-LastExit "git checkout main"
    git pull --ff-only origin main
    Check-LastExit "git pull --ff-only origin main"
    git merge --ff-only develop
    Check-LastExit "git merge develop"
    git push origin main
    Check-LastExit "git push origin main"
}

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
Write-Host "==> Tagging: $Version (push with --tags)"
if (-not $DryRun) {
    git tag $Version
    Check-LastExit "git tag"
    git push origin $Version
    Check-LastExit "git push tag"
}

# 7. Back to develop
Write-Host "==> git checkout develop"
if (-not $DryRun) {
    git checkout develop
    Check-LastExit "git checkout develop"
}

Write-Host ""
Write-Host "Done. Released $Version on main; back on develop."
if ($DryRun) { Write-Host "(dry run - nothing was changed)" }