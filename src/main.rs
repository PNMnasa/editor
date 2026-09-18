use std::{
    env,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
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
#[expect(dead_code)]
mod terminal_tools;
#[expect(dead_code)]
mod terminal_ui_tools;

use dir_info::{Entry, format_size, list_basic, list_entries};
use terminal_tools::{
    clear, enter_alt_screen, hide_cursor, leave_alt_screen, set_title, show_cursor,
};
use terminal_ui_tools::{bg_color, clear_color, fg_color, put_text};

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

/// Kết quả quét nền: generation khi bắt đầu, thư mục đã quét, và danh sách
/// đầy đủ thống kê (`None` = không tính được).
type ScanResult = Option<(usize, PathBuf, Option<Vec<Entry>>)>;

/// Trạng thái quét nền: danh sách được vẽ ngay từ `list_basic`, luồng nền
/// tính `list_entries` đầy đủ rồi ghi đè khi xong.
struct NavState {
    running_generation: Arc<AtomicUsize>,
    result: Arc<Mutex<ScanResult>>,
    computing: bool,
}

impl NavState {
    fn new() -> Self {
        Self {
            running_generation: Arc::new(AtomicUsize::new(0)),
            result: Arc::new(Mutex::new(None)),
            computing: false,
        }
    }
}

/// Vẽ danh sách `target` ngay bằng `list_basic`, đồng thời đưa việc tính
/// kích thước (đệ quy) ra luồng nền. Kết quả nền chỉ được áp dụng khi
/// generation vẫn còn hợp lệ. Trả về `true` nếu liệt kê nhanh thành công.
fn navigate(
    target: &Path,
    entries: &mut Vec<Entry>,
    state: &mut NavState,
    message: &mut String,
) -> bool {
    match list_basic(target) {
        Ok(basic) => {
            *entries = basic;
            let generation = state.running_generation.fetch_add(1, Ordering::SeqCst) + 1;
            let result = Arc::clone(&state.result);
            let own = target.to_path_buf();
            thread::spawn(move || {
                let enriched = list_entries(&own).ok();
                if let Ok(mut guard) = result.lock() {
                    *guard = Some((generation, own, enriched));
                }
            });
            state.computing = true;
            *message = "Đang tính kích thước…".to_owned();
            true
        }
        Err(err) => {
            *message = format!("Không thể đọc `{}`: {err}", target.display());
            false
        }
    }
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
        "q: quit | j/k/arraws: move | Enter: open | Backspace: up | r: refresh",
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
                    if generation == nav.running_generation.load(Ordering::SeqCst) {
                        nav.computing = false;
                        match enriched {
                            Some(new_entries) => {
                                entries = new_entries;
                                message.clear();
                            }
                            None => {
                                message = "Không tính được kích thước thư mục".to_owned();
                            }
                        }
                    }
                }
            }
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
                            if navigate(&dir, &mut entries, &mut nav, &mut message) {
                                selected = 0;
                                top = 0;
                            }
                        }
                        KeyCode::Enter => {
                            let Some(entry) = entries.get(selected) else {
                                continue;
                            };
                            if entry.is_dir {
                                let mut next = dir.clone();
                                next.push(&entry.name);
                                if navigate(&next, &mut entries, &mut nav, &mut message) {
                                    dir = next;
                                    selected = 0;
                                    top = 0;
                                }
                            } else {
                                message = format!("`{}` là file — chưa hỗ trợ mở", entry.name);
                            }
                        }
                        KeyCode::Backspace => {
                            if let Some(parent) = dir.parent() {
                                let parent = parent.to_path_buf();
                                if navigate(&parent, &mut entries, &mut nav, &mut message) {
                                    dir = parent;
                                    selected = 0;
                                    top = 0;
                                }
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
