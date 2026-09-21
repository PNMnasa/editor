<#
.SYNOPSIS
Runs the 4 CI steps: fmt, clippy, test, release build.

.DESCRIPTION
Single entry point matching .github/workflows/ci.yml so the agent loop can run
"the same as CI" with one command instead of four. Exits non-zero on first failure.

.EXAMPLE
.\scripts\ci.ps1
#>
$ErrorActionPreference = "Stop"

$steps = @(
    @{ Label = "cargo fmt --check"; Args = @("fmt", "--check") }
    @{ Label = "cargo clippy --all-targets -- -D warnings"; Args = @("clippy", "--all-targets", "--", "-D", "warnings") }
    @{ Label = "cargo test"; Args = @("test") }
    @{ Label = "cargo build --release"; Args = @("build", "--release") }
)

foreach ($s in $steps) {
    Write-Host "==> $($s.Label)"
    & cargo @($s.Args)
    if ($LASTEXITCODE -ne 0) {
        Write-Error "$($s.Label) failed (exit $LASTEXITCODE)"
        exit 1
    }
}

Write-Host ""
Write-Host "All 4 CI steps passed."