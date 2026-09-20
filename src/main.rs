use std::{
    env,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, size},
};

#[expect(dead_code)]
mod dir_info;
mod format_tools;
#[expect(dead_code)]
mod terminal_tools;
#[expect(dead_code)]
mod terminal_ui_tools;

use dir_info::{Entry, ScanOptions, list_basic, list_entries_with_checked};
use format_tools::format_size;
use terminal_tools::{
    clear, enter_alt_screen, hide_cursor, leave_alt_screen, set_title, show_cursor,
};
use terminal_ui_tools::{bg_color, clear_color, fg_color, put_text};

/// Rotating frames for the "computing" state (no external library needed).
const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

fn clip(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        text.to_owned()
    } else if max < 3 {
        chars[chars.len() - max..].iter().collect()
    } else {
        let mut out: String = chars[chars.len() - max + 3..].iter().collect();
        out.insert_str(0, "...");
        out
    }
}

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

/// Background scan result: the generation it started in, the directory it scanned,
/// and the fully enriched list (`None` = could not compute).
type ScanResult = Option<(usize, PathBuf, Option<Vec<Entry>>)>;

/// Background scan state: the list is drawn immediately from `list_basic`,
/// while a background thread runs `list_entries_with_checked` and checks the
/// `cancel` flag, then overwrites the result when it finishes.
struct NavState {
    generation: Arc<AtomicUsize>,
    cancel: Arc<AtomicBool>,
    result: Arc<Mutex<ScanResult>>,
    computing: bool,
}

impl NavState {
    fn new() -> Self {
        Self {
            generation: Arc::new(AtomicUsize::new(0)),
            cancel: Arc::new(AtomicBool::new(false)),
            result: Arc::new(Mutex::new(None)),
            computing: false,
        }
    }
}

/// Draws the `target` listing immediately via `list_basic`, pushing the
/// (recursive) size computation to a background thread. The previous scan is
/// cancelled through the `cancel` flag; a background result is applied only
/// while its generation is still current. Returns `true`
/// if the quick listing succeeded.
fn navigate(
    target: &Path,
    entries: &mut Vec<Entry>,
    state: &mut NavState,
    message: &mut String,
) -> bool {
    match list_basic(target) {
        Ok(basic) => {
            *entries = basic;
            state.cancel.store(true, Ordering::SeqCst);
            let cancel = Arc::new(AtomicBool::new(false));
            state.cancel = cancel.clone();
            let counter = Arc::clone(&state.generation);
            let generation = counter.fetch_add(1, Ordering::SeqCst) + 1;
            let result = Arc::clone(&state.result);
            let own = target.to_path_buf();
            let opts = ScanOptions::default();
            thread::spawn(
                move || match list_entries_with_checked(&own, &opts, &cancel) {
                    Ok(enriched) => {
                        if cancel.load(Ordering::SeqCst)
                            || generation != counter.load(Ordering::SeqCst)
                        {
                            return;
                        }
                        if let Ok(mut guard) = result.lock() {
                            *guard = Some((generation, own, Some(enriched)));
                        }
                    }
                    Err(_) => {
                        if cancel.load(Ordering::SeqCst)
                            || generation != counter.load(Ordering::SeqCst)
                        {
                            return;
                        }
                        if let Ok(mut guard) = result.lock() {
                            *guard = Some((generation, own, None));
                        }
                    }
                },
            );
            state.computing = true;
            *message = "Computing sizes…".to_owned();
            true
        }
        Err(err) => {
            *message = format!("Cannot read `{}`: {err}", target.display());
            false
        }
    }
}

/// Indices of the items currently shown in `entries` (hidden files filtered by
/// `show_hidden`, names by `filter`; the original sort order is kept).
fn visible_indices(entries: &[Entry], show_hidden: bool, filter: &str) -> Vec<usize> {
    let needle = filter.to_lowercase();
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            let shown = show_hidden || !entry.name.starts_with('.');
            shown && (needle.is_empty() || entry.name.to_lowercase().contains(&needle))
        })
        .map(|(index, _)| index)
        .collect()
}

fn draw(
    out: &mut dyn Write,
    view: &View<'_>,
    indices: &[usize],
    width: u16,
    height: u16,
) -> io::Result<()> {
    clear(out)?;
    set_title(out, format!("Explorer — {}", view.dir.display()))?;

    put_text(
        out,
        1,
        1,
        clip(
            &view.dir.display().to_string(),
            width.saturating_sub(8) as usize,
        ),
    )?;
    fg_color(out, 36)?;
    put_text(
        out,
        1,
        width.saturating_sub(10),
        format!("{} items", indices.len()),
    )?;
    clear_color(out)?;

    let list_area = height.saturating_sub(4) as usize;
    for (row, pos) in (view.top..view.top + list_area).enumerate() {
        let Some(&entry_index) = indices.get(pos) else {
            break;
        };
        let entry = &view.entries[entry_index];
        let y = 2 + row as u16;

        if view.selected == pos {
            bg_color(out, 44)?;
            put_text(out, y, 1, ">")?;
        } else {
            put_text(out, y, 1, " ")?;
        }
        if entry.is_dir {
            fg_color(out, 34)?;
        }
        let name_len = width.saturating_sub(14) as usize;
        put_text(out, y, 3, clip(&entry.name, name_len))?;
        clear_color(out)?;

        put_text(out, y, width.saturating_sub(8), format_size(entry.size))?;
    }

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
    let status = if view.computing {
        format!("{} {status}", view.spinner)
    } else {
        status
    };
    put_text(out, height.saturating_sub(2), 1, status)?;
    put_text(
        out,
        height.saturating_sub(1),
        1,
        "q: quit | j/k/arrows: move | PgUp/PgDn/Home/End: page | /: filter | .: hidden | Enter: open | Backspace: up | r: refresh",
    )?;
    Ok(())
}

fn restore_terminal() {
    let mut out = io::stdout();
    let _ = disable_raw_mode();
    let _ = show_cursor(&mut out);
    let _ = leave_alt_screen(&mut out);
    let _ = out.flush();
}

fn main() -> io::Result<()> {
    let start = env::args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| env::current_dir().unwrap_or_default());

    let mut dir = start;
    let mut entries = Vec::new();
    let mut selected = 0usize;
    let mut top = 0usize;
    let mut message = String::new();
    let mut nav = NavState::new();
    let mut show_hidden = false;
    let mut filter = String::new();
    let mut filtering = false;
    let mut filter_draft = String::new();
    let mut spinner = 0usize;

    navigate(&dir, &mut entries, &mut nav, &mut message);

    enable_raw_mode()?;
    let mut out = io::stdout();
    enter_alt_screen(&mut out)?;
    hide_cursor(&mut out)?;

    let guard = DropGuard;
    let result = (|| {
        loop {
            if nav.computing {
                let ready = nav
                    .result
                    .lock()
                    .map(|mut guard| guard.take())
                    .unwrap_or(None);
                if let Some((generation, _path, enriched)) = ready {
                    if generation == nav.generation.load(Ordering::SeqCst) {
                        nav.computing = false;
                        match enriched {
                            Some(new_entries) => {
                                entries = new_entries;
                                message.clear();
                            }
                            None => {
                                message = "Could not compute directory size".to_owned();
                            }
                        }
                    }
                }
            }
            let (width, height) = size()?;
            let area = height.saturating_sub(4) as usize;
            spinner = (spinner + 1) % SPINNER.len();

            let indices = visible_indices(&entries, show_hidden, &filter);
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
                dir: &dir,
                entries: &entries,
                selected,
                top,
                message: &message,
                computing: nav.computing,
                spinner: SPINNER[spinner],
                filtering,
                filter: &filter,
                filter_draft: &filter_draft,
            };
            draw(&mut out, &view, &indices, width, height)?;
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
                            if navigate(&dir, &mut entries, &mut nav, &mut message) {
                                filter.clear();
                                selected = 0;
                                top = 0;
                            }
                        }
                        KeyCode::Enter => {
                            let Some(&index) = indices.get(selected) else {
                                continue;
                            };
                            let Some(entry) = entries.get(index) else {
                                continue;
                            };
                            if entry.is_dir {
                                let mut next = dir.clone();
                                next.push(&entry.name);
                                if navigate(&next, &mut entries, &mut nav, &mut message) {
                                    dir = next;
                                    filter.clear();
                                    selected = 0;
                                    top = 0;
                                }
                            } else {
                                message = format!(
                                    "`{}` is a file — opening is not supported yet",
                                    entry.name
                                );
                            }
                        }
                        KeyCode::Backspace => {
                            if let Some(parent) = dir.parent() {
                                let parent = parent.to_path_buf();
                                if navigate(&parent, &mut entries, &mut nav, &mut message) {
                                    dir = parent;
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

#[cfg(test)]
mod tests {
    use super::clip;

    #[test]
    fn clip_empty_width_is_empty() {
        assert_eq!(clip("abc", 0), "");
    }

    #[test]
    fn clip_short_text_unchanged() {
        assert_eq!(clip("hello", 10), "hello");
    }

    #[test]
    fn clip_very_narrow_keeps_tail() {
        assert_eq!(clip("abcdefgh", 2), "gh");
        assert_eq!(clip("abcdefgh", 1), "h");
    }

    #[test]
    fn clip_long_ellipsizes_head() {
        assert_eq!(clip("abcdefghij", 8), "...fghij");
        assert_eq!(clip("abcdefghij", 5), "...ij");
    }
}
