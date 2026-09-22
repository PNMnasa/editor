//! Helper library for a directory manager: lists the files/folders of a
//! directory with size info, recursive file/folder counts and total size.
//! A pure logic module (no terminal/UI dependency), usable from both TUI and
//! GUI.
//!
//! For large directories, use `list_basic` to draw the list right away, then
//! run `list_entries_with_checked` (or `dir_stats`) on a background thread to
//! fill in sizes afterwards — avoids UI delay. The `*_checked` variants take
//! an `AtomicBool` cancel flag so the scan stops early when the user has
//! navigated elsewhere.
//!
//! Symlinks to directories count as directories (`fs::metadata` — follows the
//! symlink); broken symlinks are skipped.

use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

/// Default maximum number of items when listing one level (0 = unlimited).
pub const DEFAULT_MAX_ENTRIES: usize = 1000;
/// Default maximum subdirectory depth when computing stats (0 = do not descend).
pub const DEFAULT_MAX_DEPTH: usize = 16;

/// Scan options that cap the workload for large/deep directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanOptions {
    /// Maximum number of items listed per level; `0` = unlimited.
    pub max_entries: usize,
    /// Number of subdirectory levels entered when computing stats; `0` = direct entries only.
    pub max_depth: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_ENTRIES,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

/// Directory tree stats: number of files, directories and the total
/// (recursive) size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirStats {
    pub files: u64,
    pub dirs: u64,
    pub total_size: u64,
}

/// An entry in a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    /// Size of the file; for a directory, the recursive total size.
    pub size: u64,
    /// Recursive file count (directories only).
    pub files: u64,
    /// Recursive directory count (directories only).
    pub dirs: u64,
}

/// Internal error signalling that the scan was cancelled via the `cancel` flag.
fn cancelled_error() -> io::Error {
    io::Error::new(ErrorKind::Interrupted, "scan cancelled")
}

/// Quickly list the contents of `dir`: no recursive stats (directories come
/// back with `size/files/dirs = 0`). Use it to draw the list first, then
/// compute sizes on a background thread and overwrite.
pub fn list_basic(dir: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = collect_basic(dir, &ScanOptions::default(), None)?;
    sort_entries(&mut entries);
    Ok(entries)
}

/// List the contents of `dir` with recursive stats per directory, using the
/// default options.
pub fn list_entries(dir: &Path) -> io::Result<Vec<Entry>> {
    list_entries_with(dir, &ScanOptions::default())
}

/// List the contents of `dir` with recursive stats per directory per `opts`.
///
/// Results are sorted directories first (case-insensitive by name), then
/// files. Unreadable entries are skipped; errors on `dir` itself are
/// returned as-is. Prefer `list_basic` + a background thread for large
/// directories.
pub fn list_entries_with(dir: &Path, opts: &ScanOptions) -> io::Result<Vec<Entry>> {
    let entries = collect_basic(dir, opts, None)?;
    Ok(enrich(entries, dir, opts, None))
}

/// Like `list_entries_with` but taking a `cancel` flag: when the flag is set
/// mid-scan, returns `ErrorKind::Interrupted` so a background thread can stop
/// early instead of walking the whole tree.
pub fn list_entries_with_checked(
    dir: &Path,
    opts: &ScanOptions,
    cancel: &AtomicBool,
) -> io::Result<Vec<Entry>> {
    let entries = collect_basic(dir, opts, Some(cancel))?;
    Ok(enrich(entries, dir, opts, Some(cancel)))
}

/// Recursive stats of `dir` at the default depth.
pub fn dir_stats(dir: &Path) -> io::Result<DirStats> {
    dir_stats_with(dir, DEFAULT_MAX_DEPTH)
}

/// Recursive stats of `dir`, descending at most `max_depth` levels.
///
/// `max_depth = 0` counts only the direct entries of `dir` (does not descend).
/// Descending beyond the limit stops there; deeper directories are not added.
/// Unreadable subdirectories are skipped.
pub fn dir_stats_with(dir: &Path, max_depth: usize) -> io::Result<DirStats> {
    if !dir.is_dir() {
        return Err(io::Error::new(
            ErrorKind::NotADirectory,
            format!("{} is not a directory", dir.display()),
        ));
    }
    dir_stats_at(dir, max_depth, None)
}

fn collect_basic(
    dir: &Path,
    opts: &ScanOptions,
    cancel: Option<&AtomicBool>,
) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for item in fs::read_dir(dir)? {
        if let Some(c) = cancel {
            if c.load(Ordering::Relaxed) {
                return Err(cancelled_error());
            }
        }
        if opts.max_entries != 0 && entries.len() >= opts.max_entries {
            break;
        }
        let Ok(item) = item else {
            continue;
        };
        let Ok(meta) = fs::metadata(item.path()) else {
            continue;
        };
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        entries.push(Entry {
            name: item.file_name().to_string_lossy().into_owned(),
            is_dir,
            size,
            files: 0,
            dirs: 0,
        });
    }
    Ok(entries)
}

fn enrich(
    mut entries: Vec<Entry>,
    dir: &Path,
    opts: &ScanOptions,
    cancel: Option<&AtomicBool>,
) -> Vec<Entry> {
    if opts.max_depth > 0 {
        for entry in &mut entries {
            if entry.is_dir {
                if let Ok(stats) = dir_stats_at(&dir.join(&entry.name), opts.max_depth - 1, cancel)
                {
                    entry.size = stats.total_size;
                    entry.files = stats.files;
                    entry.dirs = stats.dirs;
                }
            }
        }
    }
    sort_entries(&mut entries);
    entries
}

fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

fn dir_stats_at(dir: &Path, depth: usize, cancel: Option<&AtomicBool>) -> io::Result<DirStats> {
    let mut stats = DirStats::default();
    for item in fs::read_dir(dir)? {
        if let Some(c) = cancel {
            if c.load(Ordering::Relaxed) {
                return Err(cancelled_error());
            }
        }
        let Ok(item) = item else {
            continue;
        };
        let Ok(meta) = fs::metadata(item.path()) else {
            continue;
        };
        if !meta.is_dir() {
            stats.files += 1;
            stats.total_size += meta.len();
            continue;
        }
        stats.dirs += 1;
        if depth > 0 {
            if let Ok(sub) = dir_stats_at(&item.path(), depth - 1, cancel) {
                stats.files += sub.files;
                stats.dirs += sub.dirs;
                stats.total_size += sub.total_size;
            }
        }
    }
    Ok(stats)
}
