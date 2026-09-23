//! GUI mode of `editor-91to9` built on `eframe`/`egui`, mirroring the explorer
//! TUI: navigation state and background size scans come from the shared
//! `browse::Browser` (immediate listing, cancelable computation thread,
//! generation-checked result), rendered in a native window.
//!
//! Enabled by the `gui` feature (default). Launch with `editor-91to9 --gui`.
//! The pure helpers (`KeyCommand`, `step_selection`) are public so the
//! selection logic stays testable without a display.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use eframe::egui;

use crate::browse::{Browser, visible_indices};
use crate::dir_info::Entry;
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

/// The explorer GUI state: a shared [`Browser`] plus the egui view state
/// (selection, filter, scrolling) that the TUI keeps in its loop.
pub struct GuiApp {
    browser: Browser,
    selected: usize,
    show_hidden: bool,
    filter: String,
    /// Row index to bring into view when the selection moved off-screen.
    scroll_to: Option<usize>,
    /// Number of visible list rows, used for PageUp/PageDown.
    page_size: usize,
    /// Focus the filter field next frame (after `/` was pressed).
    focus_filter: bool,
}

impl GuiApp {
    fn new(start_dir: PathBuf) -> Self {
        let mut browser = Browser::new();
        browser.navigate(&start_dir);
        Self {
            browser,
            selected: 0,
            show_hidden: false,
            filter: String::new(),
            scroll_to: Some(0),
            page_size: 1,
            focus_filter: false,
        }
    }

    /// Open `target` in the shared browser and reset the view state like the
    /// TUI does on navigation.
    fn navigate(&mut self, target: &Path) -> bool {
        if self.browser.navigate(target) {
            self.selected = 0;
            self.filter.clear();
            self.scroll_to = Some(0);
            true
        } else {
            false
        }
    }

    fn visible(&self) -> Vec<usize> {
        visible_indices(self.browser.entries(), self.show_hidden, &self.filter)
    }

    fn selected_entry(&self, indices: &[usize]) -> Option<&Entry> {
        let index = *indices.get(self.selected)?;
        self.browser.entries().get(index)
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
        if ctx.input(|i| i.key_pressed(egui::Key::Enter))
            && let Some(entry) = self.selected_entry(indices)
        {
            if entry.is_dir {
                let next = self.browser.dir().join(&entry.name);
                self.navigate(&next);
            } else {
                self.browser.set_message(format!(
                    "`{}` is a file — opening is not supported yet",
                    entry.name
                ));
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Backspace))
            && let Some(parent) = self.browser.dir().parent().map(Path::to_path_buf)
        {
            self.navigate(&parent);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            let current = self.browser.dir().to_path_buf();
            self.navigate(&current);
        }
    }

    /// Top toolbar: navigation buttons, hidden toggle, item count, filter.
    /// Returns the navigation target requested by a button (if any) and
    /// whether the filter field currently owns keyboard focus.
    fn show_toolbar(&mut self, ui: &mut egui::Ui, count: usize) -> (Option<PathBuf>, bool) {
        let mut target: Option<PathBuf> = None;
        let mut filter_id = None;
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("⬆ Up").clicked() {
                    target = self.browser.dir().parent().map(Path::to_path_buf);
                }
                if ui.button("↻ Refresh").clicked() {
                    target = Some(self.browser.dir().to_path_buf());
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
            filter_id.is_some_and(|id| ui.ctx().memory(|memory| memory.focused() == Some(id)));
        (target, filter_focused)
    }

    fn show_status(&mut self, ui: &mut egui::Ui) {
        let (computing, message) = (
            self.browser.is_computing(),
            self.browser.message().to_owned(),
        );
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if computing {
                    ui.add(egui::Spinner::new().size(14.0));
                }
                ui.label(message);
            });
        });
    }

    fn show_list(&mut self, ui: &mut egui::Ui) {
        let indices = self.visible();
        let count = indices.len();
        if count == 0 {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(count - 1);
        }
        let entries_empty = self.browser.entries().is_empty();
        let message = self.browser.message().to_owned();
        let mut open_dir: Option<PathBuf> = None;
        let scroll_row = self.scroll_to.take();
        egui::CentralPanel::default().show(ui, |ui| {
            if entries_empty {
                ui.label(message);
                return;
            }
            let page = (ui.available_height() / 20.0).floor().max(1.0) as usize;
            self.page_size = page;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (pos, &entry_index) in indices.iter().enumerate() {
                        let (name, is_dir, size) = {
                            let entry = &self.browser.entries()[entry_index];
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
                                open_dir = Some(self.browser.dir().join(&name));
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
    /// Non-painting per-frame work, called before `ui` (and also while the
    /// window is hidden): keep polling the size scan and the repaint loop
    /// alive until it finishes, since egui only repaints on events.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.browser.poll();
        if self.browser.is_computing() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    /// Build the whole UI tree inside the root `Ui`: top toolbar panel first,
    /// bottom status panel, then the `CentralPanel` (the scrollable list).
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let indices = self.visible();
        let count = indices.len();
        let (target, filter_focused) = self.show_toolbar(ui, count);
        self.handle_keys(&ctx, &indices, filter_focused);
        if let Some(target) = target {
            self.navigate(&target);
        }
        self.show_status(ui);
        self.show_list(ui);
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
        Box::new(move |_cc| Ok(Box::new(GuiApp::new(start_dir)))),
    )
}
