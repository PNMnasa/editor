//! Command-line entry shared by every build mode: resolves the start
//! directory and picks the frontend to launch. Compiled unconditionally so the
//! binary keeps working however it was built:
//!
//! - `tui` feature only — the explorer TUI runs; `--gui` explains itself
//! - `gui` feature only — the GUI runs; there is no TUI to fall back to
//! - `full` (both features, the default) — `--gui` selects the GUI,
//!   otherwise the TUI runs

use std::{env, io, path::PathBuf};

/// The start directory: the first argument that is a directory, or the current
/// directory when no directory was given.
fn resolve_start_dir(args: &[String]) -> PathBuf {
    args.iter()
        .map(PathBuf::from)
        .find(|p| p.is_dir())
        .unwrap_or_else(|| env::current_dir().unwrap_or_default())
}

/// Launch the explorer from the full `env::args()` iterator (argument 0, the
/// program name, is skipped).
pub fn run(args: impl Iterator<Item = String>) -> io::Result<()> {
    let args: Vec<String> = args.skip(1).collect();
    let start = resolve_start_dir(&args);

    #[cfg(feature = "tui")]
    let tui_requested = !args.iter().any(|arg| arg == "--gui");

    #[cfg(feature = "tui")]
    if tui_requested {
        return run_tui(start);
    }

    run_gui(start)
}

#[cfg(feature = "tui")]
fn run_tui(start: PathBuf) -> io::Result<()> {
    crate::tui::run(start)
}

/// Launch the GUI mode. Returns `io::Result<()>` so it fits in `run`'s
/// signature; the GUI's own return type is `eframe::Result`.
#[cfg(feature = "gui")]
fn run_gui(start: PathBuf) -> io::Result<()> {
    match crate::gui::run(start) {
        Ok(()) => Ok(()),
        Err(err) => {
            eprintln!("GUI error: {err}");
            std::process::exit(1);
        }
    }
}

/// The `gui` feature is what compiles the `egui`/`eframe` dependency; without
/// it the flag (or a build without any TUI to run) explains itself instead of
/// failing silently.
#[cfg(not(feature = "gui"))]
fn run_gui(_start: PathBuf) -> io::Result<()> {
    eprintln!("GUI mode is not available in this build — compile with the `gui` feature");
    std::process::exit(1);
}
