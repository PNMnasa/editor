# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- GUI mode alongside the TUI: `editor-91to9 --gui` opens an eframe/egui explorer mirroring the TUI (list with background size computation, `j/k`/arrow + mouse navigation, `/` filter, `.`/`h` hidden toggle, `Enter`/double-click to open, `Backspace`/`Up` to go up, `r` to refresh). Built on eframe 0.36 (MSRV 1.95) behind the `gui` build-mode feature; new `src/gui.rs` with pure, tested helpers (`visible_indices`, `step_selection`, `KeyCommand`)
- TUI: quick navigation keys `PgUp`/`PgDn`/`Home`/`End`, filter the list with `/` (Enter applies / Esc cancels) and toggle hidden files with `.` or `h`
- Spinner for the "computing sizes" state while the background scan is still running
- `benches/scan.rs`: dependency-free manual micro-benchmark — measures `list_entries_with` and cross-checks `format_size` against `u64::ilog2`, an `if/else` chain, a multiply loop, and (Windows only) `StrFormatByteSizeW` (speed-only; output is not compared because of the decimal base 1000) — (`cargo bench --bench scan`)
- Newcomer-friendly docs: `README.md` gains a Usage section (run command + key bindings) and the naming of `editor-91to9` is explained; `docs/INSTALL.md` uses the real clone URL; added `CODE_OF_CONDUCT.md` and GitHub issue/pull-request templates; the `Cargo.toml` description now matches the current tool (editor support is explicitly listed as planned)

### Changed

- GUI upgraded to eframe/egui 0.36.2 (from 0.32): the default `wgpu` renderer is replaced by the `glow` (OpenGL) renderer — wgpu's Windows build currently fails on a `windows-core` 0.61/0.62 version clash. The GUI now implements the new `eframe::App` model (`ui(&mut egui::Ui, …)` for painting plus `logic(...)` for the non-painting scan polling/repaint scheduling that keeps running while the window is hidden), and the layout uses the unified `egui::Panel` type instead of `TopBottomPanel`/`CentralPanel` over a `Context`. MSRV raised 1.85 → 1.95 (eframe 0.36 needs rustc ≥ 1.95)
- `browse`: new `src/browse.rs` module — one shared navigation core for both front ends (`Browser`: immediate listing + cancelable background size scan + `poll` + messages, plus `visible_indices`); the TUI and the GUI now share a single startup/navigation implementation instead of each keeping its own copy
- Build modes through Cargo features: `tui` (crossterm), `gui` (eframe), `full` = both (default); pick one with `--no-default-features --features tui|gui`. `crossterm` is now optional, so a GUI-only build no longer pulls it in. The `--gui` dispatch moved out of `tui.rs` into an always-compiled `src/cli.rs` — `cli::run` resolves the start directory and picks the frontend for the current build mode
- `tui`: the whole explorer moved out of `main` into `src/tui.rs` — `main` is now a no-logic entry point that just calls `editor_91to9::cli::run(env::args())`, which dispatches between the TUI and the `--gui` mode
- `dir_info`: the background thread now uses `list_entries_with_checked` with a cancel flag — the previous scan stops early when navigating to another folder, and only the current generation writes its result
- `clip`: truncates by display columns instead of character count — East-Asian wide/fullwidth glyphs and emoji count as two columns, combining marks as none; wide file names no longer overflow the row layout
- `main`: the terminal state (raw mode, cursor, alt screen) is restored even when the app panics mid-run — a crash no longer leaves the shell unusable
- `dir_info`: symlinks to folders count as folders, broken symlinks are skipped (uses `fs::metadata` — follows the symlink)
- `format_size`: automatically picks the unit up to exabyte (B, K, M, G, T, P, E) via the highest set bit (`leading_zeros`) instead of an `if/else` chain — `u64` cannot represent ZB/YB
- Whole project unified to English: docs, changelog, code comments, crate description, terminal UI messages and bench output are now written in English (previously mixed Vietnamese/English)
- `main`: the TUI now redraws only the lines that actually changed (title/item-count line, individual list rows, status line) instead of clearing and redrawing the whole screen every frame — the spinner ticks touch only the status line, selection changes touch only the affected rows, and once a background scan finishes only the rows whose displayed sizes changed are updated
- Restructured into a library + binary: new `src/lib.rs` exposes `dir_info`/`format_tools`/`terminal_tools`/`terminal_ui_tools` as public modules; `main.rs` now imports them from the crate. All tests moved from inline `#[cfg(test)]` blocks into `tests/` as integration tests (`tests/dir_info.rs`, `tests/format_tools.rs`, `tests/terminal_ui_tools.rs`); `clip` moved from `main.rs` to `format_tools` so it stays testable, and the duplicated `format_size_units` test was dropped

### Fixed

- `dir_info`: `collect_basic`/`dir_stats_at` use `fs::metadata(item.path())` (follows symlinks) instead of `DirEntry::metadata()`
- Security: terminal escape injection — untrusted text (file names, paths, messages) is sanitized at the render boundary (`put_text` and the window title are the two sinks); C0 controls become caret notation (`cat -v` style), DEL `^?`, C1 controls U+FFFD — a file named e.g. `\x1b]0;…` can no longer inject ANSI/OSC sequences into the terminal

## [0.1.1]

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
