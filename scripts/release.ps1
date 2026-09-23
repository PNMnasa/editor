<#
.SYNOPSIS
Quick release pipeline: analyze changes, auto-commit (grouped), push main, tag.

.DESCRIPTION
One command that:
  1. Runs the 4 CI steps (fmt, clippy, test, release build) - can be skipped.
  2. Analyzes the pending changes and commits them with a Conventional Commits message,
     splitting into multiple commits per area (ci / docs / src / tooling / other).
  3. Pulls origin/main (fast-forward) and pushes main.
  4. Creates a tag (auto patch bump from the current Cargo.toml version when -Version is omitted; the version is written back to Cargo.toml/Cargo.lock so the tag always matches) and pushes it.

.PARAMETER Message
Optional explicit commit message. When set, everything is committed in a single commit instead of
grouped analysis.

.PARAMETER Version
Tag name, e.g. "v0.2.0". Default: auto patch bump from the current Cargo.toml version. When the Cargo.toml version does not match, it is bumped (and Cargo.lock synced) so the tag always matches the package version.

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

$repoRoot = Split-Path -Parent $PSScriptRoot

function Get-PackageVersion {
    $match = Select-String -LiteralPath (Join-Path $repoRoot "Cargo.toml") -Pattern '^version = "([^"]+)"'
    if (-not $match) { Fail "Could not read the version from Cargo.toml" }
    $match.Matches[0].Groups[1].Value
}

function Set-PackageVersion([string]$Version) {
    $path = Join-Path $repoRoot "Cargo.toml"
    $content = Get-Content -LiteralPath $path -Raw
    $updated = [regex]::Replace($content, '(?m)^version = "[^"]*"', "version = `"$Version`"", 1)
    if ($updated -eq $content) { Fail "Could not find the package version line in Cargo.toml" }
    Set-Content -LiteralPath $path -Value $updated -NoNewline
}

function Bump-PatchVersion([string]$Version) {
    $parts = ($Version -replace "^v", "") -split "\."
    while ($parts.Count -lt 3) { $parts += "0" }
    try {
        "v$($parts[0]).$($parts[1]).$([int]$parts[2] + 1)"
    } catch {
        Fail "Could not bump version '$Version'."
    }
}

# 1. Branch guard
$current = git branch --show-current
Check-LastExit "git branch --show-current"
if ($current -ne "main") {
    Fail "Must be on branch 'main' (currently: '$current')"
}

# 2. Determine the release version and sync Cargo.toml
$cargoVersion = Get-PackageVersion
if (-not $Version) {
    $lastTag = git describe --tags --abbrev=0 2>$null
    if ($LASTEXITCODE -ne 0) { $lastTag = $null }
    if ($lastTag -and $lastTag -eq "v$cargoVersion") {
        $Version = Bump-PatchVersion $cargoVersion
    } else {
        $Version = "v$cargoVersion"
    }
}
if ($Version -notmatch '^v\d+\.\d+\.\d+$') {
    Fail "Version must look like 'vX.Y.Z' (got '$Version')"
}
$target = $Version.Substring(1)
if ($target -ne $cargoVersion) {
    Write-Host "==> Setting Cargo.toml version: $cargoVersion -> $target"
    if (-not $DryRun) {
        Set-PackageVersion $target
        & cargo check
        if ($LASTEXITCODE -ne 0) { Fail "cargo check failed after the version bump (Cargo.lock sync)" }
    }
} else {
    Write-Host "==> Cargo.toml already at $cargoVersion"
}

# 3. CI checks
if ($DryRun) {
    Write-Host "==> [dryrun] would run the 4 CI steps"
} elseif (-not $SkipChecks) {
    Write-Host "==> Running CI checks..."
    & (Join-Path $PSScriptRoot "ci.ps1")
    if ($LASTEXITCODE -ne 0) { Fail "CI checks failed (see scripts/ci.ps1)" }
} else {
    Write-Host "==> Skipping CI checks."
}

# 4. Commit pending changes
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

# 5. Pull origin/main (fast-forward) and push
Write-Host "==> git pull --ff-only origin main; git push origin main"
if (-not $DryRun) {
    git pull --ff-only origin main
    Check-LastExit "git pull --ff-only origin main"
    git push origin main
    Check-LastExit "git push origin main"
}

# 6. Tag and push
Write-Host "==> Tagging: $Version"
if (-not $DryRun) {
    git tag $Version
    Check-LastExit "git tag"
    git push origin $Version
    Check-LastExit "git push tag"
}

Write-Host ""
Write-Host "Done. Released $Version on main."
if ($DryRun) { Write-Host "(dry run - nothing was changed)" }