# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- TUI: quick navigation keys `PgUp`/`PgDn`/`Home`/`End`, filter the list with `/` (Enter applies / Esc cancels) and toggle hidden files with `.` or `h`
- Spinner for the "computing sizes" state while the background scan is still running
- Unit tests for `clip` (`src/main.rs`) and `format_size` continuing into terabyte units
- `benches/scan.rs`: dependency-free manual micro-benchmark — measures `list_entries_with` and cross-checks `format_size` against `u64::ilog2`, an `if/else` chain, a multiply loop, and (Windows only) `StrFormatByteSizeW` (speed-only; output is not compared because of the decimal base 1000) — (`cargo bench --bench scan`)
- CI: job `config-check` runs `actionlint` (validates `ci.yml`/`release.yml`), `scripts/check_opencode.sh`/`.ps1` (opencode.json allowlist invariants), syntax-checks `scripts/*.sh` and parses `scripts/*.ps1`
- Markdown lint: job `config-check` uses `rumdl` (a Rust markdown linter, rule IDs follow the markdownlint MDxxx standard) on every `.md` file in the repo — `.rumdl.toml` documents each choice; also fixes leftover violations (MD034/MD047/MD012)
- `rust-toolchain.toml`: pins the `stable` toolchain with `clippy`/`rustfmt` components so fmt/clippy stay consistent between CI and local
- `scripts/ci.ps1`: wraps the 4 CI steps into a single command for the agent loop / release
- `scripts/snapshot-ref.ps1`: stores a snapshot ref (`git update-ref refs/backup/<name>`) before destructive operations
- opencode skill `.opencode/skill/release/SKILL.md` documenting the release workflow
- `release.yml`: job `verify` automatically blocks when the tag does not match the `Cargo.toml` version or `CHANGELOG.md` is missing a `## [vX]` entry, and uses that very entry as the GitHub release body
- opencode `/verify` command (`.opencode/command/verify.md`): runs the CI script, the allowlist check and shows git status/log in a single quick check
- `scripts/*.sh`: native bash variants (Linux/macOS) for `ci`, `check_opencode`, `snapshot-ref`, `release` — identical behavior to the `.ps1` variants
- `.markdownlint.json`: mirrors `.rumdl.toml` (MD013 off) for the `markdownlint` linter used by editors (Neovim…) — editors filter MDxxx warnings exactly per repo convention

### Changed

- `dir_info`: the background thread now uses `list_entries_with_checked` with a cancel flag — the previous scan stops early when navigating to another folder, and only the current generation writes its result
- `dir_info`: symlinks to folders count as folders, broken symlinks are skipped (uses `fs::metadata` — follows the symlink)
- `format_size`: automatically picks the unit up to exabyte (B, K, M, G, T, P, E) via the highest set bit (`leading_zeros`) instead of an `if/else` chain — `u64` cannot represent ZB/YB
- `opencode.json`: allows running the `ci`/`check_opencode`/`snapshot-ref` scripts (both `.ps1` and `.sh`) and `git update-ref refs/backup/*` without asking
- `release.ps1` reuses `scripts/ci.ps1` for the CI check step; `release.sh` reuses `scripts/ci.sh`
- `ci.yml`: `config-check` runs native `scripts/check_opencode.sh` and syntax-checks `scripts/*.sh` with bash; `scripts/*.ps1` are still parsed with pwsh on the runner
- Scripts moved to **native builds per OS** (`scripts/*.ps1` for Windows, `scripts/*.sh` for Linux/macOS) instead of forcing pwsh everywhere — pick the right variant for the OS, no extra runtime needed
- `.opencode/agent/reviewer.md`: the reviewer runs the real CI script and reviews the diff against project conventions instead of only reading the code

### Fixed

- `dir_info`: test `dir_stats_not_a_directory` no longer uses a name that easily collides in CWD — uses a PID-based temp path
- `CONTRIBUTING.md`: clippy instructions now use `--all-targets -- -D warnings` to match CI
- `dir_info`: `collect_basic`/`dir_stats_at` use `fs::metadata(item.path())` (follows symlinks) instead of `DirEntry::metadata()` — on Linux/macOS the old version does not follow symlinks, so the `symlinked_dir_counts_as_dir` test failed on CI (Windows still passed due to different reparse-point semantics)
- `release.yml`: fixed the wrong built binary name (`editor` → `editor-91to9`) and the trigger now only fires when a `v*` tag is created instead of on every `main` push

## [0.1.1] - 2026-09-20

### Changed

- Explorer lists files/folders immediately, then computes sizes/counts in a background thread (`dir_info::list_basic`) and overwrites when done — no more delay when opening large folders
- Simplified the workflow to a single `main` branch: dropped `develop`, `release.ps1` runs straight from `main` (pull/push `main` + tag), CI no longer triggers on `develop`, and the `repository` in `Cargo.toml` no longer carries the `/tree/develop` suffix

### Added

- Initial project skeleton: `src/` layout, `Cargo.toml`, `docs/INSTALL.md`
- Docs: `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`
- GPL-3.0 license
- CI workflow (GitHub Actions) for Windows, Linux, macOS
- Basic explorer TUI in `src/main.rs`
- Module `src/dir_info.rs`: lists files/folders with recursive size and counts, total size, `max_entries`/`max_depth` limits, `format_size` helper
- Quick release script `scripts/release.ps1`: commit, push `main`, create a tag in one command
