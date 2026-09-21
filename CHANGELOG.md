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
- `.markdownlint.json`: mirrors `.rumdl.toml` (MD013 off) for the `markdownlint` linter used by editors (Neovim…) — editors filter MDxxx warnings exactly per repo convention
- `rust-toolchain.toml`: pins the `stable` toolchain with `clippy`/`rustfmt` components so fmt/clippy stay consistent between CI and local

### Changed

- `dir_info`: the background thread now uses `list_entries_with_checked` with a cancel flag — the previous scan stops early when navigating to another folder, and only the current generation writes its result
- `format_size`: automatically picks the unit up to exabyte (B, K, M, G, T, P, E) via the highest set bit (`leading_zeros`) instead of an `if/else` chain — `u64` cannot represent ZB/YB
- `opencode.json`: allows running the `ci`/`check_opencode`/`snapshot-ref` scripts (both `.ps1` and `.sh`) and `git update-ref refs/backup/*` without asking

### Fixed

- `dir_info`: test `dir_stats_not_a_directory` no longer uses a name that easily collides in CWD — uses a PID-based temp path
- `CONTRIBUTING.md`: clippy instructions now use `--all-targets -- -D warnings` to match CI

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
