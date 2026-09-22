# Editor for anything

Editor for anything — coding, design (planned). Today the project ships an
explorer TUI: files and folders are listed with their sizes, computed in the
background so the UI never visibly lags. Code/image editing is on the roadmap,
not yet implemented.

The package and binary are named `editor-91to9` (historical project name); the
library is `editor_91to9`.

## Usage

Run from the repo root (optionally pointing at a directory):

```sh
cargo run
cargo run -- /some/path
```

Keys:

- `q` / `Esc` — quit
- `j` / `k` or arrow keys — move the selection
- `PgUp` / `PgDn` / `Home` / `End` — page around the list
- `/` — filter by name (Enter applies, Esc cancels)
- `.` or `h` — toggle hidden files
- `Enter` — open the selected folder
- `Backspace` — go up one level
- `r` — refresh the current folder

## Features

### Completed

- File/folder manager: view the name and size of files & folders
- UI optimization: heavy computation runs in the background so the UI never visibly lags

### Roadmap

- Editor: code in many languages, images (PNG), other formats
- GUI alongside the TUI
- Extras: coding hints & CLI, AI (MCP, Skills)

### Built for

- Humans, agents

## Install

- Build & install: [docs/INSTALL.md](docs/INSTALL.md)

## Contributing

- Issues and feature requests: <https://github.com/PNMnasa/editor/issues>
- Guidance: [CONTRIBUTING.md](CONTRIBUTING.md) · [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

## Tech

- **Rust** — single dependency: `crossterm`
- **Own ANSI terminal layer** on top of crossterm for the TUI; pure logic lives in the `editor_91to9` library
- **Native for any OS**
