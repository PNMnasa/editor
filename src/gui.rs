//! GUI mode of `editor-91to9` built on `eframe`/`egui`, mirroring the explorer
//! TUI: files/folders are listed immediately with their file size, recursive
//! directory sizes are computed on a background thread (cancelled on every
//! navigation, only the current generation writes its result) and overwrite
//! the list when done.
//!
//! Enabled by the `gui` feature (default). Launch with `editor-91to9 --gui`.
//! The pure helpers (`visible_indices`, `step_selection`, `KeyCommand`) are
//! public so the selection/filter logic stays testable without a display.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
};

use eframe::egui;

use crate::dir_info::{Entry, ScanOptions, list_basic, list_entries_with_checked};
use crate::format_tools::{clip, format_size};

/// Folder names are tinted blue, mirroring the TUI's directory color.
const DIR_COLOR: egui::Color32 = egui::Color32::from_rgb(70, 150, 255);

/// Keyboard movement commands, mirroring the TUI navigation keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCommand {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

impl KeyCommand {
    /// Map this frame's pressed keys to a movement command, if any.
    fn from_egui(ctx: &egui::Context) -> Option<Self> {
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::K)) {
            Some(Self::Up)
        } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::J))
        {
            Some(Self::Down)
        } else if ctx.input(|i| i.key_pressed(egui::Key::PageUp)) {
            Some(Self::PageUp)
        } else if ctx.input(|i| i.key_pressed(egui::Key::PageDown)) {
            Some(Self::PageDown)
        } else if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
            Some(Self::Home)
        } else if ctx.input(|i| i.key_pressed(egui::Key::End)) {
            Some(Self::End)
        } else {
            None
        }
    }
}

/// Move `selected` by `cmd` inside a list of `count` visible rows, paging by
/// `area` rows; guarded against empty lists and off-by-one at both ends.
pub fn step_selection(selected: usize, count: usize, area: usize, cmd: KeyCommand) -> usize {
    if count == 0 {
        return 0;
    }
    match cmd {
        KeyCommand::Up => selected.saturating_sub(1),
        KeyCommand::Down => (selected + 1).min(count - 1),
        KeyCommand::PageUp => selected.saturating_sub(area.max(1)),
        KeyCommand::PageDown => selected.saturating_add(area).min(count - 1),
        KeyCommand::Home => 0,
        KeyCommand::End => count - 1,
    }
}

/// Indices of the entries currently visible (hidden files filtered by
/// `show_hidden`, names by the lowercase `filter`; original sort order kept).
/// Mirrors the TUI's `visible_indices`.
pub fn visible_indices(entries: &[Entry], show_hidden: bool, filter: &str) -> Vec<usize> {
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

/// Background scan result: generation, scanned directory, full stats list
/// (`None` = could not be computed).
type ScanResult = Option<(usize, PathBuf, Option<Vec<Entry>>)>;

/// Background scan state shared with the computation thread: the list is drawn
/// immediately from `list_basic`, the thread computes
/// `list_entries_with_checked` while checking the `cancel` flag (each
/// navigation sets the flag of the previous scan so it stops early) and only
/// the current generation overwrites the result.
#[derive(Default)]
struct ScanState {
    generation: Arc<AtomicUsize>,
    cancel: Arc<AtomicBool>,
    result: Arc<Mutex<ScanResult>>,
    computing: bool,
}

/// The explorer GUI state, mirroring the TUI's view/navigation model.
pub struct GuiApp {
    ctx: egui::Context,
    dir: PathBuf,
    entries: Vec<Entry>,
    selected: usize,
    show_hidden: bool,
    filter: String,
    message: String,
    scan: ScanState,
    /// Row index to bring into view when the selection moved off-screen.
    scroll_to: Option<usize>,
    /// Number of visible list rows, used for PageUp/PageDown.
    page_size: usize,
    /// Focus the filter field next frame (after `/` was pressed).
    focus_filter: bool,
}

impl GuiApp {
    fn new(cc: &eframe::CreationContext<'_>, start_dir: PathBuf) -> Self {
        let mut app = Self {
            ctx: cc.egui_ctx.clone(),
            dir: start_dir,
            entries: Vec::new(),
            selected: 0,
            show_hidden: false,
            filter: String::new(),
            message: String::new(),
            scan: ScanState::default(),
            scroll_to: Some(0),
            page_size: 1,
            focus_filter: false,
        };
        let dir = app.dir.clone();
        app.navigate(&dir);
        app
    }

    /// Start (or cancel/restart) the background size scan for `self.dir`.
    fn start_scan(&mut self) {
        self.scan.cancel.store(true, Ordering::SeqCst);
        let cancel = Arc::new(AtomicBool::new(false));
        self.scan.cancel = Arc::clone(&cancel);
        let counter = Arc::clone(&self.scan.generation);
        let generation = counter.fetch_add(1, Ordering::SeqCst) + 1;
        let result = Arc::clone(&self.scan.result);
        let own = self.dir.clone();
        let ctx = self.ctx.clone();
        let opts = ScanOptions::default();
        thread::spawn(move || {
            let outcome = list_entries_with_checked(&own, &opts, &cancel);
            if cancel.load(Ordering::SeqCst) || generation != counter.load(Ordering::SeqCst) {
                return;
            }
            if let Ok(mut guard) = result.lock() {
                *guard = Some((generation, own, outcome.ok()));
            }
            ctx.request_repaint();
        });
        self.scan.computing = true;
        self.message = "Computing sizes…".to_owned();
    }

    /// Draw the list for `target` immediately, start the background scan and
    /// reset selection/filter/scroll like the TUI does on navigation.
    fn navigate(&mut self, target: &Path) -> bool {
        match list_basic(target) {
            Ok(basic) => {
                self.dir = target.to_path_buf();
                self.entries = basic;
                self.selected = 0;
                self.filter.clear();
                self.scroll_to = Some(0);
                self.start_scan();
                true
            }
            Err(err) => {
                self.message = format!("Cannot read `{}`: {err}", target.display());
                false
            }
        }
    }

    /// Apply a finished background scan if it is still the current generation.
    fn poll_scan(&mut self) {
        if !self.scan.computing {
            return;
        }
        let ready = self.scan.result.lock().map(|mut guard| guard.take()).ok();
        if let Some(Some((generation, _path, enriched))) = ready {
            if generation != self.scan.generation.load(Ordering::SeqCst) {
                return;
            }
            self.scan.computing = false;
            match enriched {
                Some(entries) => {
                    self.entries = entries;
                    self.message.clear();
                }
                None => self.message = "Could not compute directory size".to_owned(),
            }
        }
    }

    fn visible(&self) -> Vec<usize> {
        visible_indices(&self.entries, self.show_hidden, &self.filter)
    }

    fn selected_entry(&self, indices: &[usize]) -> Option<&Entry> {
        let index = *indices.get(self.selected)?;
        self.entries.get(index)
    }

    fn move_selection(&mut self, cmd: KeyCommand, count: usize) {
        let next = step_selection(self.selected, count, self.page_size, cmd);
        if next != self.selected {
            self.selected = next;
            self.scroll_to = Some(next);
        }
    }

    /// Handle the navigation keys (Enter/Backspace/`r`/movement). Navigation
    /// is ignored while the filter field is being edited, and `/` sends focus
    /// to it instead.
    fn handle_keys(&mut self, ctx: &egui::Context, indices: &[usize], filter_focused: bool) {
        if filter_focused {
            return;
        }
        let count = indices.len();
        if ctx.input(|i| i.key_pressed(egui::Key::Slash)) {
            self.focus_filter = true;
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Period) || i.key_pressed(egui::Key::H)) {
            self.show_hidden = !self.show_hidden;
            self.selected = 0;
            self.scroll_to = Some(0);
        }
        if let Some(cmd) = KeyCommand::from_egui(ctx) {
            self.move_selection(cmd, count);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(entry) = self.selected_entry(indices) {
                if entry.is_dir {
                    let next = self.dir.join(&entry.name);
                    self.navigate(&next);
                } else {
                    self.message =
                        format!("`{}` is a file — opening is not supported yet", entry.name);
                }
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Backspace)) {
            if let Some(parent) = self.dir.parent().map(Path::to_path_buf) {
                self.navigate(&parent);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            let current = self.dir.clone();
            self.navigate(&current);
        }
    }

    /// Top toolbar: navigation buttons, hidden toggle, item count, filter.
    /// Returns the navigation target requested by a button (if any) and
    /// whether the filter field currently owns keyboard focus.
    fn show_toolbar(&mut self, ctx: &egui::Context, count: usize) -> (Option<PathBuf>, bool) {
        let mut target: Option<PathBuf> = None;
        let mut filter_id = None;
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("⬆ Up").clicked() {
                    target = self.dir.parent().map(Path::to_path_buf);
                }
                if ui.button("↻ Refresh").clicked() {
                    target = Some(self.dir.clone());
                }
                ui.separator();
                if ui
                    .checkbox(&mut self.show_hidden, "Show hidden files")
                    .changed()
                {
                    self.selected = 0;
                    self.scroll_to = Some(0);
                }
                ui.separator();
                ui.label(format!("{count} items"));
            });
            ui.horizontal(|ui| {
                ui.label("Filter:");
                let response = ui
                    .add(egui::TextEdit::singleline(&mut self.filter).hint_text("filter by name…"));
                if response.changed() {
                    self.selected = 0;
                    self.scroll_to = Some(0);
                }
                if self.focus_filter {
                    response.request_focus();
                    self.focus_filter = false;
                }
                if ui.button("✕").on_hover_text("Clear filter").clicked() {
                    self.filter.clear();
                    self.selected = 0;
                    self.scroll_to = Some(0);
                }
                filter_id = Some(response.id);
            });
        });
        let filter_focused =
            filter_id.is_some_and(|id| ctx.memory(|memory| memory.focused() == Some(id)));
        (target, filter_focused)
    }

    fn show_status(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.scan.computing {
                    ui.add(egui::Spinner::new().size(14.0));
                }
                ui.label(self.message.clone());
            });
        });
    }

    fn show_list(&mut self, ctx: &egui::Context) {
        let indices = self.visible();
        let count = indices.len();
        if count == 0 {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(count - 1);
        }
        let mut open_dir: Option<PathBuf> = None;
        let scroll_row = self.scroll_to.take();
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.entries.is_empty() {
                ui.label(self.message.clone());
                return;
            }
            let page = (ui.available_height() / 20.0).floor().max(1.0) as usize;
            self.page_size = page;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (pos, &entry_index) in indices.iter().enumerate() {
                        let (name, is_dir, size) = {
                            let entry = &self.entries[entry_index];
                            (clip(&entry.name, 200), entry.is_dir, entry.size)
                        };
                        let text = if is_dir {
                            egui::RichText::new(&name).color(DIR_COLOR)
                        } else {
                            egui::RichText::new(&name)
                        };
                        ui.horizontal(|ui| {
                            let is_selected = self.selected == pos;
                            let response = ui.selectable_label(is_selected, text);
                            if response.clicked() {
                                self.selected = pos;
                            }
                            if response.double_clicked() && is_dir {
                                open_dir = Some(self.dir.join(&name));
                            }
                            if scroll_row == Some(pos) {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(egui::RichText::new(format_size(size)).weak());
                                },
                            );
                        });
                    }
                });
        });
        if let Some(target) = open_dir {
            self.navigate(&target);
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_scan();
        let indices = self.visible();
        let count = indices.len();
        let (target, filter_focused) = self.show_toolbar(ctx, count);
        self.handle_keys(ctx, &indices, filter_focused);
        if let Some(target) = target {
            self.navigate(&target);
        }
        self.show_status(ctx);
        self.show_list(ctx);
    }
}

/// Launch the GUI explorer in a native window at `start_dir`.
pub fn run(start_dir: PathBuf) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Editor (GUI)",
        options,
        Box::new(move |cc| Ok(Box::new(GuiApp::new(cc, start_dir)))),
    )
}
