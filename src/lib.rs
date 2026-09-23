//! Shared library of `editor-91to9`: pure logic modules used by both the TUI
//! binary and the tests/benchmarks, the shared browsing core (`browse`), the
//! frontends (`tui` behind the `tui` feature, `gui` behind the `gui` feature)
//! and the unconditional `cli` entry that picks between them. Build modes:
//! default is `full` (both), or pick one with
//! `--no-default-features --features tui|gui`. The binary stays a thin entry
//! point in `main.rs`.

pub mod browse;
pub mod cli;
pub mod dir_info;
pub mod format_tools;
#[cfg(feature = "gui")]
pub mod gui;
pub mod terminal_tools;
pub mod terminal_ui_tools;
#[cfg(feature = "tui")]
pub mod tui;
