//! Shared library of `editor-91to9`: pure logic modules used by both the TUI
//! binary and the tests/benchmarks, plus the optional GUI mode (`gui`
//! feature). The binary stays the entry point in `main.rs`.

pub mod dir_info;
pub mod format_tools;
#[cfg(feature = "gui")]
pub mod gui;
pub mod terminal_tools;
pub mod terminal_ui_tools;
