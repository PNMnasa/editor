//! Explorer TUI: terminal rendering and key handling that drive the shared
//! `browse::Browser`. Compiled only with the `tui` build-mode feature; the
//! `cli` module picks between this and the GUI. `main` stays a thin wrapper
//! around `cli::run`.

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, size},
};

use crate::browse::{Browser, visible_indices};
use crate::dir_info::Entry;
use crate::format_tools::{clip, format_size};
use crate::terminal_tools::{
    clear, clear_line, enter_alt_screen, goto, hide_cursor, leave_alt_screen, set_title,
    show_cursor,
};
use crate::terminal_ui_tools::{bg_color, clear_color, fg_color, put_text, sanitize_for_terminal};

/// Spinner frames for the "computing" state (no external library needed).
const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

struct View<'a> {
    dir: &'a Path,
    entries: &'a [Entry],
    selected: usize,
    top: usize,
    message: &'a str,
    computing: bool,
    spinner: char,
    filtering: bool,
    filter: &'a str,
    filter_draft: &'a str,
}

/// The text drawn on one list row; compared across frames so only rows whose
/// content (name, size, folder style or selection marker) actually changed
/// need to be redrawn.
#[derive(Clone, PartialEq)]
struct Cell {
    name: String,
    size: String,
    is_dir: bool,
    selected: bool,
}

/// Snapshot of what is currently on screen. `sync` diffs the new frame against
/// it and only rewrites the parts that changed (title/count line, individual
/// list rows, status line) instead of clearing and redrawing the whole screen.
#[derive(Default)]
struct Screen {
    dir: String,
    count: usize,
    width: u16,
    height: u16,
    rows: Vec<Option<Cell>>,
    status: String,
}

/// The clipped directory path shown on the title line.
fn dir_caption(dir: &Path, width: u16) -> String {
    clip(&dir.display().to_string(), width.saturating_sub(8) as usize)
}

/// What should be rendered on the list row `pos` (its offset into the visible
/// list, starting at `view.top`), or `None` when there is no entry at that
/// position.
fn cell_for(view: &View<'_>, indices: &[usize], width: u16, pos: usize) -> Option<Cell> {
    let entry_index = *indices.get(view.top + pos)?;
    let entry = &view.entries[entry_index];
    Some(Cell {
        name: clip(&entry.name, width.saturating_sub(14) as usize),
        size: format_size(entry.size),
        is_dir: entry.is_dir,
        selected: view.selected == pos,
    })
}

/// Redraw the first line: directory path on the left, item count on the right.
fn render_title(out: &mut dyn Write, width: u16, dir: &Path, count: usize) -> io::Result<()> {
    goto(out, 1, 1)?;
    clear_line(out)?;
    put_text(out, 1, 1, dir_caption(dir, width))?;
    fg_color(out, 36)?;
    put_text(out, 1, width.saturating_sub(10), format!("{count} items"))?;
    clear_color(out)
}

/// Redraw the last line: the keyboard help (drawn once per full redraw).
fn render_help(out: &mut dyn Write, height: u16) -> io::Result<()> {
    let y = height.saturating_sub(1);
    goto(out, y, 1)?;
    clear_line(out)?;
    put_text(
        out,
        y,
        1,
        "q: quit | j/k/arrows: move | PgUp/PgDn/Home/End: page | /: filter | .: hidden | Enter: open | Backspace: up | r: refresh",
    )
}

/// Redraw the contents of one list row (marker, name, size).
fn render_list_row(out: &mut dyn Write, y: u16, cell: &Cell, width: u16) -> io::Result<()> {
    goto(out, y, 1)?;
    clear_line(out)?;
    if cell.selected {
        bg_color(out, 44)?;
        put_text(out, y, 1, ">")?;
    } else {
        put_text(out, y, 1, " ")?;
    }
    if cell.is_dir {
        fg_color(out, 34)?;
    }
    put_text(out, y, 3, &cell.name)?;
    clear_color(out)?;
    put_text(out, y, width.saturating_sub(8), &cell.size)
}

/// Clear the entire contents of a list row whose entry no longer exists.
fn clear_row(out: &mut dyn Write, y: u16) -> io::Result<()> {
    goto(out, y, 1)?;
    clear_line(out)
}

/// Compose the second-to-last line: computing spinner, message, filter state.
fn status_line(view: &View<'_>) -> String {
    let status = if view.filtering {
        format!("Filter (Enter=apply, Esc=cancel): {}", view.filter_draft)
    } else {
        let mut status = view.message.to_owned();
        if !view.filter.is_empty() {
            if !status.is_empty() {
                status.push_str(" · ");
            }
            status.push_str(&format!("filter \"{}\"", view.filter));
        }
        status
    };
    if view.computing {
        format!("{} {status}", view.spinner)
    } else {
        status
    }
}

/// Redraw the second-to-last line (cleared before writing so a shorter
/// previous status cannot leave trailing characters).
fn render_status(out: &mut dyn Write, height: u16, status: &str) -> io::Result<()> {
    let y = height.saturating_sub(2);
    goto(out, y, 1)?;
    clear_line(out)?;
    put_text(out, y, 1, status)
}

/// Draw the screen, redrawing only the parts that changed since the previous
/// call. A full redraw (whole `clear`) happens once at startup, on resize, on
/// navigation (new directory) and otherwise only the title/count line, the
/// list rows whose content changed and the status line are rewritten — so the
/// spinner ticks and, once a scan finishes, only the rows whose displayed
/// sizes changed are updated.
fn sync(
    out: &mut dyn Write,
    view: &View<'_>,
    indices: &[usize],
    width: u16,
    height: u16,
    screen: &mut Screen,
) -> io::Result<()> {
    let dir = view.dir.display().to_string();
    let count = indices.len();
    let status = status_line(view);
    let area = height.saturating_sub(4) as usize;

    if screen.rows.is_empty()
        || screen.width != width
        || screen.height != height
        || screen.dir != dir
    {
        clear(out)?;
        set_title(out, sanitize_for_terminal(&format!("Explorer — {dir}")))?;
        render_title(out, width, view.dir, count)?;
        render_help(out, height)?;
        render_status(out, height, &status)?;
        screen.rows.clear();
        for pos in 0..area {
            let cell = cell_for(view, indices, width, pos);
            if let Some(cell) = &cell {
                render_list_row(out, 2 + pos as u16, cell, width)?;
            }
            screen.rows.push(cell);
        }
    } else {
        if screen.count != count {
            render_title(out, width, view.dir, count)?;
        }
        for pos in 0..area {
            let cell = cell_for(view, indices, width, pos);
            if cell != *screen.rows.get(pos).unwrap_or(&None) {
                match &cell {
                    Some(cell) => render_list_row(out, 2 + pos as u16, cell, width)?,
                    None => clear_row(out, 2 + pos as u16)?,
                }
            }
            screen.rows[pos] = cell;
        }
        if screen.status != status {
            render_status(out, height, &status)?;
        }
    }

    screen.dir = dir;
    screen.count = count;
    screen.width = width;
    screen.height = height;
    screen.status = status;
    Ok(())
}

fn restore_terminal() {
    let mut out = io::stdout();
    let _ = disable_raw_mode();
    let _ = show_cursor(&mut out);
    let _ = leave_alt_screen(&mut out);
    let _ = out.flush();
}

/// Run the TUI explorer at `start` in raw mode, restoring the terminal on
/// exit (including panics).
pub fn run(start: PathBuf) -> io::Result<()> {
    {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let mut out = io::stdout();
            let _ = disable_raw_mode();
            let _ = show_cursor(&mut out);
            let _ = leave_alt_screen(&mut out);
            let _ = out.flush();
            default_hook(info);
        }));
    }

    let mut browser = Browser::new();
    browser.navigate(&start);
    let mut selected = 0usize;
    let mut top = 0usize;
    let mut show_hidden = false;
    let mut filter = String::new();
    let mut filtering = false;
    let mut filter_draft = String::new();
    let mut spinner = 0usize;
    let mut screen = Screen::default();

    enable_raw_mode()?;
    let mut out = io::stdout();
    enter_alt_screen(&mut out)?;
    hide_cursor(&mut out)?;

    let guard = DropGuard;
    let result = (|| {
        loop {
            browser.poll();
            let (width, height) = size()?;
            let area = height.saturating_sub(4) as usize;
            spinner = (spinner + 1) % SPINNER.len();

            let indices = visible_indices(browser.entries(), show_hidden, &filter);
            let count = indices.len();
            if count == 0 {
                selected = 0;
                top = 0;
            } else {
                selected = selected.min(count - 1);
                top = selected
                    .saturating_sub(area.saturating_sub(1))
                    .min(count.saturating_sub(area));
            }

            let view = View {
                dir: browser.dir(),
                entries: browser.entries(),
                selected,
                top,
                message: browser.message(),
                computing: browser.is_computing(),
                spinner: SPINNER[spinner],
                filtering,
                filter: &filter,
                filter_draft: &filter_draft,
            };
            sync(&mut out, &view, &indices, width, height, &mut screen)?;
            out.flush()?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if filtering {
                        match key.code {
                            KeyCode::Char(c) => filter_draft.push(c),
                            KeyCode::Backspace => {
                                filter_draft.pop();
                            }
                            KeyCode::Enter => {
                                filter = filter_draft.clone();
                                filtering = false;
                                selected = 0;
                            }
                            KeyCode::Esc => filtering = false,
                            _ => {}
                        }
                        continue;
                    }
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Up | KeyCode::Char('k') => {
                            selected = selected.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if selected + 1 < count {
                                selected += 1;
                            }
                        }
                        KeyCode::PageUp => {
                            selected = selected.saturating_sub(area);
                        }
                        KeyCode::PageDown => {
                            selected = selected.saturating_add(area).min(count.saturating_sub(1));
                        }
                        KeyCode::Home => selected = 0,
                        KeyCode::End => selected = count.saturating_sub(1),
                        KeyCode::Char('/') => {
                            filtering = true;
                            filter_draft = filter.clone();
                        }
                        KeyCode::Char('.') | KeyCode::Char('h') => {
                            show_hidden = !show_hidden;
                            selected = 0;
                        }
                        KeyCode::Char('r') => {
                            let current = browser.dir().to_path_buf();
                            if browser.navigate(&current) {
                                filter.clear();
                                selected = 0;
                                top = 0;
                            }
                        }
                        KeyCode::Enter => {
                            let Some(&index) = indices.get(selected) else {
                                continue;
                            };
                            let Some(entry) = browser.entries().get(index) else {
                                continue;
                            };
                            if entry.is_dir {
                                let mut next = browser.dir().to_path_buf();
                                next.push(&entry.name);
                                if browser.navigate(&next) {
                                    filter.clear();
                                    selected = 0;
                                    top = 0;
                                }
                            } else {
                                browser.set_message(format!(
                                    "`{}` is a file — opening is not supported yet",
                                    entry.name
                                ));
                            }
                        }
                        KeyCode::Backspace => {
                            if let Some(parent) = browser.dir().parent().map(Path::to_path_buf) {
                                if browser.navigate(&parent) {
                                    filter.clear();
                                    selected = 0;
                                    top = 0;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    })();
    drop(guard);
    restore_terminal();
    result
}

struct DropGuard;

impl Drop for DropGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}
