# AGENTS.md

Instructions for agents working in the `editor` project.

## Context

- Read `README.md` for project goals, `docs/INSTALL.md` for build/install steps.
- Single Rust crate (`editor-91to9`), edition 2024, MSRV 1.85, only dependency is `crossterm`. No `[[bin]]` entry, so the binary is named after the package (`editor-91to9(.exe)`).
- Toolchain pinned to `stable` via `rust-toolchain.toml` (with `clippy`/`rustfmt` components) — `cargo` installs it on demand; the MSRV 1.85 in `Cargo.toml` is the minimum floor, not the toolchain in use.
- Goal is TUI + GUI, licensed GPL-3.0 — contributions must be compatible.

## Common commands

- Build: `cargo build`; release: `cargo build --release`
- Compile check: `cargo check`
- Run tests: `cargo test`
- Run one test: `cargo test dir_info::tests::test_function_name`
- Micro-benchmark (dependency-free): `cargo bench --bench scan`
- Format: `cargo fmt --check` (check), `cargo fmt` (fix)
- Lint: `cargo clippy --all-targets -- -D warnings` — `CONTRIBUTING.md` omits `--all-targets`; only the `--all-targets` form matches CI/`release.ps1`.
- Markdown lint: `cargo install rumdl` (one-time) then `rumdl check .` — MDxxx rule IDs (markdownlint standard); config `.rumdl.toml` documents every deviation. Runs in CI via job `config-check`. Editors that use the `markdownlint` (MDxxx) linter read `.markdownlint.json`, which mirrors `.rumdl.toml` — keep the two in sync when changing rule choices.
- Run the program: `cargo run`
- Run all 4 CI steps exactly like `ci.yml`: `scripts/ci.ps1` (Windows) / `scripts/ci.sh` (Linux/macOS) — fmt --check → clippy `-D warnings` → test → build --release, stops at first failure.
- Each script in `scripts/` has a **native build per OS**: `.ps1` runs with PowerShell on Windows, `.sh` with bash on Linux/macOS — identical behavior. Always use the variant for the current OS (e.g. `scripts/ci.ps1` on Windows, `scripts/ci.sh` on Linux/macOS); when editing logic, keep both variants in sync.
- Quick pre-commit check inside opencode: type `/verify` (runs `scripts/ci.ps1` + `scripts/check_opencode.ps1` + shows git status/log).

## Structure & gotchas

- Modules in `src/` (organized by role, file count not fixed — growing):
  - `main.rs` — binary entry, explorer TUI; draws the list immediately with `list_basic`, computes sizes on a background thread (`list_entries_with_checked`; the previous scan is cancelled via `AtomicBool` on every navigation, and only the current generation writes its result) then overwrites when done. Keys: `j/k`/arrows, `PgUp`/`PgDn`/`Home`/`End`, `/` to filter, `.`/`h` to toggle hidden files, `Enter`/`Backspace`/`r` to navigate/refresh, `q`/`Esc` to quit. Unit tests for `clip` live here.
  - `dir_info.rs` — file/folder listing and size stats (folders-first sort; symlinks to dirs count as dirs via `fs::metadata`; unreadable entries are skipped). **Holds the majority of the tests**; they create real temp dirs, no fixtures or external services.
  - `format_tools.rs` — number/string formatting (`format_size`, autonomous unit pick up to exabyte: B/K/M/G/T/P/E). Unit tests for `format_size` live here.
  - `terminal_tools.rs` — ANSI helpers.
  - `terminal_ui_tools.rs` — text/color/box drawing.
- `benches/scan.rs` — dependency-free manual micro-benchmark (includes `src/dir_info.rs` and `src/format_tools.rs` via `#[path]`, so it is NOT the crate's private module); run with `cargo bench --bench scan` (extra args: `cargo bench --bench scan -- <scans> <format-reps>`). It also cross-checks `format_size` against `u64::ilog2`, if/else-chain, multiply-loop and — on Windows only — `StrFormatByteSizeW` (decimal base, output not compared).
- Modules not fully consumed (`dir_info`, `terminal_tools`, `terminal_ui_tools`) carry `#[expect(dead_code)]` at `mod` level in `main.rs`; `format_tools` does not because `format_size` is used directly. If you use the whole public API of a module that carries `#[expect]`, it becomes `unfulfilled_lint_expectations` and clippy `-D warnings` fails — keep or drop the `#[expect]` per module.
- `opencode.json` (and `.opencode/agent/reviewer.md`) configures OpenCode with a **safe-command allowlist** strategy: catch-all `ask` is FIRST, non-state-changing read/check commands (`git status/log/diff/show/fetch`, `cargo check/test/fmt/clippy/build`, `Get-ChildItem`, `Test-Path`, …) and `git add`/`git commit` are auto-run so the agent loop never blocks; risky commands (`git push`, `git reset`, `git checkout/switch`, `git restore`, `git clean/rm`, `git branch -D`, `git tag -d`, `git commit --amend`, `cargo publish`, `cargo run`, `Remove-Item`, `rm`, …) sit at the END → always ask the user. Key order matters: last matching rule wins. CI enforces this invariant with `scripts/check_opencode.ps1` (job `config-check`) — don't break it: `*` must stay first and must not be `allow`. Config is only loaded at startup — after editing `opencode.json` you must restart opencode for changes to take effect.
- Inside `.opencode/`, only `agent/reviewer.md`, `command/verify.md` and `skill/release/SKILL.md` are tracked; `package*.json` and `node_modules` are plugin scratch, gitignored via `.opencode/.gitignore` (root `.gitignore` only has `/target`) — do not commit them. Use the release skill when the user asks for a release — see `.opencode/skill/release/SKILL.md`.
- CI (`.github/workflows/ci.yml`): job `config-check` validates opencode.json allowlist, bash/PowerShell script syntax, actionlint, **and lints Markdown with `rumdl`** (`.rumdl.toml`); the compile matrix — `fmt --check`, `clippy --all-targets -- -D warnings`, `test`, `build --release` — runs on Windows, Linux, macOS.
- Release workflow (`.github/workflows/release.yml`): triggered by tag `v*` on `main`, builds release on 3 platforms and creates a GitHub release (binaries renamed `editor-linux` / `editor-windows.exe` / `editor-macos`) — the `verify` job blocks if the tag doesn't match the `Cargo.toml` version or `CHANGELOG.md` is missing a `## [vX]` entry, and uses that entry as the GitHub release body.

## Important conventions

- **Do not touch `src/` unless directly asked** — the codebase is being kept frozen.
- **Experiment on local branches, never push**: create a dedicated local branch per experiment (prefix like `dev/<name>` or `agents/<name>`) from `main`; only push a branch to GitHub when explicitly asked so the remote stays clean. If you just need to keep a commit as a marker without a branch: `scripts/snapshot-ref.ps1` (Windows) / `scripts/snapshot-ref.sh` (Linux/macOS) with `-Name <name>` (kept at `refs/backup/<name>`).
- Every change must pass all 4 CI steps (bundled by `scripts/ci.ps1` / `scripts/ci.sh` for the current OS, matching `ci.yml`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`.
- Follow `CONTRIBUTING.md` when contributing.
- Log notable changes in `CHANGELOG.md` (Keep a Changelog format).
- Before committing/pushing you may run the reviewer agent (`.opencode/agent/reviewer.md`) — it runs the native CI script for the current OS (Windows: `scripts/ci.ps1`; Linux/macOS: `scripts/ci.sh`) and reviews the diff against project conventions; it never edits files.
- Docs (including `CHANGELOG.md`) and code identifiers are written in English.

## Quick release

- One command for the whole pipeline: `scripts/release.ps1` (Windows) / `scripts/release.sh` (Linux/macOS) plus a commit message — runs the 4 CI steps, commits cleanly (or auto-groups by area if no message is given), pulls/pushes `main`, creates a tag (auto patch bump if `-Version` is omitted), and pushes the tag (triggers the release workflow).
- **Must be on `main`** — the script aborts otherwise.
- Options: `-SkipChecks` (skip CI steps); `-Version v0.2.0` (pin a specific tag); `-DryRun` (print changes only, no commit).
- Details: see the `.opencode/skill/release/SKILL.md` skill.
