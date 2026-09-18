use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, size},
};

#[expect(dead_code)]
mod terminal_tools;
#[expect(dead_code)]
mod terminal_ui_tools;

use terminal_tools::{
    clear, enter_alt_screen, hide_cursor, leave_alt_screen, set_title, show_cursor,
};
use terminal_ui_tools::{bg_color, clear_color, fg_color, put_text};

struct Entry {
    name: String,
    is_dir: bool,
    size: u64,
}

fn list_entries(dir: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for item in fs::read_dir(dir)? {
        let item = item?;
        let file_type = item.file_type()?;
        let name = item.file_name().to_string_lossy().into_owned();
        let size = if file_type.is_file() {
            item.metadata()?.len()
        } else {
            0
        };
        entries.push(Entry {
            name,
            is_dir: file_type.is_dir(),
            size,
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    if bytes >= KB * KB * KB {
        format!("{:.1}G", bytes as f64 / (KB * KB * KB) as f64)
    } else if bytes >= KB * KB {
        format!("{:.1}M", bytes as f64 / (KB * KB) as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}

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
}

fn draw(out: &mut dyn Write, view: &View<'_>, width: u16, height: u16) -> io::Result<()> {
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
        format!("{} items", view.entries.len()),
    )?;
    clear_color(out)?;

    let list_area = height.saturating_sub(4) as usize;
    for (row, index) in (view.top..view.top + list_area).enumerate() {
        let Some(entry) = view.entries.get(index) else {
            break;
        };
        let y = 2 + row as u16;

        if view.selected == index {
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

    put_text(out, height.saturating_sub(2), 1, view.message)?;
    put_text(
        out,
        height.saturating_sub(1),
        1,
        "q: quit | j/k/arrows: move | Enter: open | Backspace: up | r: refresh",
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
    let mut entries = list_entries(&dir)?;
    let mut selected = 0usize;
    let mut top = 0usize;
    let mut message = String::new();

    enable_raw_mode()?;
    let mut out = io::stdout();
    enter_alt_screen(&mut out)?;
    hide_cursor(&mut out)?;

    let guard = DropGuard;
    let result = (|| {
        loop {
            let (width, height) = size()?;
            let view = View {
                dir: &dir,
                entries: &entries,
                selected,
                top,
                message: &message,
            };
            draw(&mut out, &view, width, height)?;
            out.flush()?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Up | KeyCode::Char('k') => {
                            selected = selected.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if selected + 1 < entries.len() {
                                selected += 1;
                            }
                        }
                        KeyCode::Char('r') => {
                            entries = list_entries(&dir)?;
                            selected = 0;
                            top = 0;
                            message.clear();
                        }
                        KeyCode::Enter => {
                            let Some(entry) = entries.get(selected) else {
                                continue;
                            };
                            if entry.is_dir {
                                let mut next = dir.clone();
                                next.push(&entry.name);
                                entries = list_entries(&next)?;
                                dir = next;
                                selected = 0;
                                top = 0;
                                message.clear();
                            } else {
                                message = format!(
                                    "`{}` is a file — opening is not supported yet",
                                    entry.name
                                );
                            }
                        }
                        KeyCode::Backspace => {
                            if let Some(parent) = dir.parent() {
                                dir = parent.to_path_buf();
                                entries = list_entries(&dir)?;
                                selected = 0;
                                top = 0;
                                message.clear();
                            }
                        }
                        _ => {}
                    }
                }
            }

            let area = height.saturating_sub(4) as usize;
            let len = entries.len();
            if len == 0 {
                top = 0;
            } else {
                top = selected
                    .saturating_sub(area.saturating_sub(1))
                    .min(len.saturating_sub(area));
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
