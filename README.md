# Editor for anything

Editor for anything — coding, design (planned). Today the project ships an
explorer with a TUI and a GUI: files and folders are listed with their sizes,
computed in the background so the UI never visibly lags. Code/image editing is
on the roadmap, not yet implemented.

The package and binary are named `editor-91to9` (historical project name); the
library is `editor_91to9`.

## Usage

Run from the repo root (optionally pointing at a directory):

```sh
cargo run                # explorer TUI
cargo run -- --gui       # explorer GUI (native window; optional `gui` feature)
cargo run -- --gui /some/path
cargo run -- /some/path  # TUI starting at a directory
```

Build modes are chosen with Cargo features (`tui`, `gui`, `full` = both):

```sh
cargo build                                       # full — both frontends (default)
cargo build --no-default-features --features tui  # TUI only
cargo build --no-default-features --features gui  # GUI only
```

TUI keys:

- `q` / `Esc` — quit
- `j` / `k` or arrow keys — move the selection
- `PgUp` / `PgDn` / `Home` / `End` — page around the list
- `/` — filter by name (Enter applies, Esc cancels)
- `.` or `h` — toggle hidden files
- `Enter` — open the selected folder
- `Backspace` — go up one level
- `r` — refresh the current folder

GUI controls mirror the TUI: the same keys work (`j/k` + arrows to move,
`Home`/`End`/`PgUp`/`PgDn` to page, `/` to filter, `.`/`h` to toggle hidden
files, `Enter`/`Backspace` to navigate, `r` to refresh, `Esc`/close to quit)
next to mouse support — click to select, double-click a folder to open it —
plus an `Up`, `Refresh` and filter toolbar.

## Features

### Completed

- File/folder manager: view the name and size of files & folders
- UI optimization: heavy computation runs in the background so the UI never visibly lags (TUI and GUI)
- Explorer GUI alongside the TUI (`--gui`, eframe/egui), sharing the same listing/scans

### Roadmap

- Editor: code in many languages, images (PNG), other formats
- Extras: coding hints & CLI, AI (MCP, Skills)

### Built for

- Humans, agents

## Install

- Build & install: [docs/INSTALL.md](docs/INSTALL.md)

## Contributing

- Issues and feature requests: <https://github.com/PNMnasa/editor/issues>
- Guidance: [CONTRIBUTING.md](CONTRIBUTING.md) · [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

## Tech

- **Rust** — two optional dependencies: `crossterm` (TUI) and `eframe 0.36` (GUI) — picked by the `tui`/`gui`/`full` build modes
- **Own ANSI terminal layer** on top of crossterm for the TUI; pure logic lives in the `editor_91to9` library
- **Native for any OS**
