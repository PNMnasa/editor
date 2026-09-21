---
description: Runs the full verify suite and summarizes state (CI checks, allowlist invariants, git status, changelog).
---

Run the verification suite and report a compact verdict:

1. Run the native CI script for this OS — `scripts/ci.ps1` on Windows, `scripts/ci.sh` on Linux/macOS. It runs the same 4 steps as `.github/workflows/ci.yml` (fmt --check, clippy --all-targets -- -D warnings, test, release build) and stops on the first failure, so report which step passed/failed.
2. Run the native allowlist check — `scripts/check_opencode.ps1` on Windows, `scripts/check_opencode.sh` on Linux/macOS — to validate the opencode.json allowlist invariants.
3. Run `git status --porcelain` and `git log --oneline -5`.
4. Summarize: PASS/FAIL per step, whether the working tree is clean, and — if there are uncommitted changes — remind that notable changes must get a `CHANGELOG.md` `[Unreleased]` entry before committing.

Do not modify any files. End with a clear verdict (READY / NOT READY) and the failing steps, if any.