# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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