---
name: release
description: Guide for running the editor-91to9 release flow through scripts/release.ps1 and scripts/ci.ps1. Use when the user asks to ship a new release, tag, commit+push for a release, or asks how to release.
---

# Release editor-91to9

`scripts/release.ps1` (Windows) / `scripts/release.sh` (Linux/macOS) handles the whole pipeline from the `main` branch (4 CI steps → grouped commits by area → pull/push `main` → tag → push the tag, which triggers the release workflow).

## Mandatory rules

- **Must be on the `main` branch** — the script aborts (`Fail`) otherwise.
- Before running, check the working tree with `git status --porcelain`; the script only commits what is currently on `main`.
- Do not run a release on your own unless the user asks — it is a risky operation and always needs confirmation.

## Using release.ps1

| Requirement | Command |
| --- | --- |
| Full release (4 CI steps + commit + push + tag) | `scripts/release.ps1 "feat: ..."` (Windows) / `scripts/release.sh "feat: ..."` (Linux/macOS) |
| Auto-group commits by area (ci/docs/src/tooling/other) | release script without a message |
| Preview only, no changes | add `-SkipChecks -DryRun` (ps1) / `--skip-checks --dry-run` (sh) |
| Pin a specific tag | `-Version v0.2.0` (ps1) / `--version v0.2.0` (sh) |
| Skip the 4 CI steps | `-SkipChecks` (ps1) / `--skip-checks` (sh) |

The tag auto-bumps `patch` from the last tag when no `-Version` is passed (based on `git describe --tags --abbrev=0`).

## Without the script

1. Run the 4 CI steps: `scripts/ci.ps1` (Windows) / `scripts/ci.sh` (Linux/macOS)
2. Commit using Conventional Commits, grouped by area.
3. `git pull --ff-only origin main` then `git push origin main`.
4. `git tag v<version>` and `git push origin v<version>` — the tag triggers the release workflow, which creates the GitHub release (binaries for 3 platforms, renamed `editor-linux` / `editor-windows.exe` / `editor-macos`).

## After tagging

- The GitHub release is created automatically by the workflow — no manual work needed.
- `cargo publish` to crates.io is a **separate** step, only done when the user asks, and remember to bump the version in `Cargo.toml` to match the tag.
