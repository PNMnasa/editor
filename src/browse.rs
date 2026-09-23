//! Shared browsing core used by both the TUI and the GUI: points at a
//! directory, lists it immediately with `list_basic` and computes the
//! recursive sizes on a cancelable background thread, then applies the result
//! once the current generation is done. `visible_indices` (the common
//! hidden/filter projection) lives here too, so the two front ends share one
//! implementation instead of keeping their own copies.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
};

use crate::dir_info::{Entry, ScanOptions, list_basic, list_entries_with_checked};

/// Finished scan outcome written by the background thread: the generation it
/// started in and the full stats list (`None` = could not be computed).
type ScanResult = Option<(usize, Option<Vec<Entry>>)>;

/// Browsing state shared between a front end and its scan thread: the list is
/// drawn immediately from `list_basic`, the thread computes
/// `list_entries_with_checked` while checking the `cancel` flag (each new
/// `navigate` sets the flag of the previous scan so it stops early) and only
/// the current generation overwrites the result.
#[derive(Default)]
pub struct Browser {
    dir: PathBuf,
    entries: Vec<Entry>,
    generation: Arc<AtomicUsize>,
    cancel: Arc<AtomicBool>,
    result: Arc<Mutex<ScanResult>>,
    computing: bool,
    message: String,
}

impl Browser {
    /// A browser pointed at nothing; call [`navigate`](Self::navigate) before
    /// use.
    pub fn new() -> Self {
        Self::default()
    }

    /// Point the browser at `target`: list it immediately with `list_basic`,
    /// then move the recursive size computation to a background thread. The
    /// previous scan is cancelled via the `cancel` flag; background results
    /// are applied only while the generation is still valid. Returns `true`
    /// on a successful quick listing.
    pub fn navigate(&mut self, target: &Path) -> bool {
        match list_basic(target) {
            Ok(basic) => {
                self.dir = target.to_path_buf();
                self.entries = basic;
                self.cancel.store(true, Ordering::SeqCst);
                let cancel = Arc::new(AtomicBool::new(false));
                self.cancel = cancel.clone();
                let counter = Arc::clone(&self.generation);
                let generation = counter.fetch_add(1, Ordering::SeqCst) + 1;
                let result = Arc::clone(&self.result);
                let own = self.dir.clone();
                let opts = ScanOptions::default();
                thread::spawn(move || {
                    let outcome = list_entries_with_checked(&own, &opts, &cancel);
                    if cancel.load(Ordering::SeqCst) || generation != counter.load(Ordering::SeqCst)
                    {
                        return;
                    }
                    if let Ok(mut guard) = result.lock() {
                        *guard = Some((generation, outcome.ok()));
                    }
                });
                self.computing = true;
                self.message = "Computing sizes…".to_owned();
                true
            }
            Err(err) => {
                self.message = format!("Cannot read `{}`: {err}", target.display());
                false
            }
        }
    }

    /// Apply a finished scan if it is still the current generation; the
    /// entries and message are updated in place when it is.
    pub fn poll(&mut self) {
        if !self.computing {
            return;
        }
        let ready = self.result.lock().map(|mut guard| guard.take()).ok();
        if let Some(Some((generation, enriched))) = ready {
            if generation != self.generation.load(Ordering::SeqCst) {
                return;
            }
            self.computing = false;
            match enriched {
                Some(entries) => {
                    self.entries = entries;
                    self.message.clear();
                }
                None => self.message = "Could not compute directory size".to_owned(),
            }
        }
    }

    /// The directory the browser is currently showing.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The current listing (basic while a scan is computing, fully enriched
    /// once it finished).
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Whether a background size scan is still running.
    pub fn is_computing(&self) -> bool {
        self.computing
    }

    /// The status message to show (computing spinner text, current error,
    /// or empty once everything is done).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Show a transient message (e.g. UI feedback such as "opening files is
    /// not supported yet") in place of the scan status until the next
    /// interaction overwrites it.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

/// Indices of the entries currently visible in `entries` (hidden files
/// filtered by `show_hidden`, names by the lowercase `filter`; original sort
/// order kept). Shared by the TUI and the GUI.
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
