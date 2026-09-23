# INSTALL

## System requirements

- Rust toolchain **1.95** or later (compatible with edition 2024) — install via [rustup](https://rustup.rs/)
- Operating systems: Windows, Linux, macOS and other terminal-capable platforms

## Building from source

1. Clone the project:

   ```sh
   git clone https://github.com/PNMnasa/editor.git
   cd editor
   ```

2. Build the release binary:

   ```sh
   cargo build --release
   ```

3. Run:

   ```sh
   ./target/release/editor-91to9      # explorer TUI
   ./target/release/editor-91to9 --gui  # explorer GUI
   ```

The GUI (`--gui`) needs a graphical session. The default mode (`full`) builds
both frontends; build only one with
`cargo build --release --no-default-features --features tui` (TUI only) or
`cargo build --release --no-default-features --features gui` (GUI only). On
Linux, building/running the GUI uses the same presentation libraries as
`eframe`; make sure the X11/Wayland development libraries are available (e.g.
on Debian/Ubuntu
`libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev`).
