//! Helper library for a directory manager: lists the files/folders of a
//! directory with size info, recursive file/folder counts and total size.
//! A pure logic module (no terminal/UI dependency), usable from both TUI and
//! GUI.
//!
//! Example:
//!
//! ```ignore
//! use std::path::Path;
//! use editor::dir_info;
//!
//! let entries = dir_info::list_entries(Path::new("."))?;
//! for entry in &entries {
//!     let stats = if entry.is_dir {
//!         format!("{} file, {} folder", entry.files, entry.dirs)
//!     } else {
//!         dir_info::format_size(entry.size)
//!     };
//!     println!("{} ({})", entry.name, stats);
//! }
//! # Ok::<(), std::io::Error>(())
//! ```

use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
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

/// Directory tree stats: number of files, directories and the total (recursive) size.
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

/// List the contents of `dir` with recursive stats per directory, using the
/// default options.
pub fn list_entries(dir: &Path) -> io::Result<Vec<Entry>> {
    list_entries_with(dir, &ScanOptions::default())
}

/// List the contents of `dir` with recursive stats per directory per `opts`.
///
/// Results are sorted directories first (case-insensitive by name), then
/// files. Unreadable entries are skipped; errors on `dir` itself are
/// returned as-is.
pub fn list_entries_with(dir: &Path, opts: &ScanOptions) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for item in fs::read_dir(dir)? {
        if opts.max_entries != 0 && entries.len() >= opts.max_entries {
            break;
        }
        let Ok(item) = item else {
            continue;
        };
        let name = item.file_name().to_string_lossy().into_owned();
        let Ok(file_type) = item.file_type() else {
            continue;
        };
        let is_dir = file_type.is_dir();
        let (size, files, dirs) = if is_dir {
            if opts.max_depth == 0 {
                (0, 0, 0)
            } else {
                match dir_stats_at(&item.path(), opts.max_depth - 1) {
                    Ok(stats) => (stats.total_size, stats.files, stats.dirs),
                    Err(_) => (0, 0, 0),
                }
            }
        } else {
            let size = item.metadata().map(|meta| meta.len()).unwrap_or(0);
            (size, 0, 0)
        };
        entries.push(Entry {
            name,
            is_dir,
            size,
            files,
            dirs,
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
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
    dir_stats_at(dir, max_depth)
}

/// Format a byte count into a readable unit (B, K, M, G — base 1024).
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    if bytes as f64 >= KB * KB * KB {
        format!("{:.1}G", bytes as f64 / (KB * KB * KB))
    } else if bytes as f64 >= KB * KB {
        format!("{:.1}M", bytes as f64 / (KB * KB))
    } else if bytes as f64 >= KB {
        format!("{:.1}K", bytes as f64 / KB)
    } else {
        format!("{bytes}B")
    }
}

fn dir_stats_at(dir: &Path, depth: usize) -> io::Result<DirStats> {
    let mut stats = DirStats::default();
    for item in fs::read_dir(dir)? {
        let Ok(item) = item else {
            continue;
        };
        let Ok(file_type) = item.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            if let Ok(meta) = item.metadata() {
                stats.files += 1;
                stats.total_size += meta.len();
            }
            continue;
        }
        stats.dirs += 1;
        if depth > 0 {
            if let Ok(sub) = dir_stats_at(&item.path(), depth - 1) {
                stats.files += sub.files;
                stats.dirs += sub.dirs;
                stats.total_size += sub.total_size;
            }
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> io::Result<PathBuf> {
        let root =
            std::env::temp_dir().join(format!("editor_dir_info_{}_{}", std::process::id(), name));
        if root.exists() {
            fs::remove_dir_all(&root)?;
        }
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    /// Sample tree:
    /// ```
    /// root/
    ///   dir1/
    ///     f1.txt (10B)
    ///     sub/
    ///       f2.txt (7B)
    ///   dir2/        (empty)
    ///   a.txt        (3B)
    /// ```
    fn make_tree(root: &Path) -> io::Result<()> {
        fs::create_dir_all(root.join("dir1/sub"))?;
        fs::create_dir_all(root.join("dir2"))?;
        fs::write(root.join("dir1/f1.txt"), vec![b'x'; 10])?;
        fs::write(root.join("dir1/sub/f2.txt"), vec![b'x'; 7])?;
        fs::write(root.join("a.txt"), vec![b'y'; 3])?;
        Ok(())
    }

    #[test]
    fn list_sorted_dirs_first_and_stats() -> io::Result<()> {
        let root = temp_root("list_sorted")?;
        make_tree(&root)?;
        let entries = list_entries(&root)?;
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["dir1", "dir2", "a.txt"]);

        let dir1 = &entries[0];
        assert!(dir1.is_dir);
        assert_eq!(dir1.files, 2);
        assert_eq!(dir1.dirs, 1);
        assert_eq!(dir1.size, 17);

        let dir2 = &entries[1];
        assert_eq!((dir2.files, dir2.dirs, dir2.size), (0, 0, 0));

        let file = &entries[2];
        assert!(!file.is_dir);
        assert_eq!((file.files, file.dirs, file.size), (0, 0, 3));
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn dir_stats_full() -> io::Result<()> {
        let root = temp_root("stats_full")?;
        make_tree(&root)?;
        let stats = dir_stats(&root)?;
        assert_eq!(stats.files, 3);
        assert_eq!(stats.dirs, 3);
        assert_eq!(stats.total_size, 20);
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn dir_stats_depth_limits() -> io::Result<()> {
        let root = temp_root("stats_depth")?;
        make_tree(&root)?;
        let depth0 = dir_stats_with(&root, 0)?;
        assert_eq!((depth0.files, depth0.dirs, depth0.total_size), (1, 2, 3));
        let depth1 = dir_stats_with(&root, 1)?;
        assert_eq!((depth1.files, depth1.dirs, depth1.total_size), (2, 3, 13));
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn max_entries_caps_listing() -> io::Result<()> {
        let root = temp_root("max_entries")?;
        for name in ["a.txt", "b.txt", "c.txt"] {
            fs::write(root.join(name), vec![b'x'; 4])?;
        }
        let opts = ScanOptions {
            max_entries: 2,
            max_depth: 1,
        };
        let entries = list_entries_with(&root, &opts)?;
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| !e.is_dir));
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn max_entries_zero_is_unlimited() -> io::Result<()> {
        let root = temp_root("max_entries_zero")?;
        make_tree(&root)?;
        let opts = ScanOptions {
            max_entries: 0,
            max_depth: 1,
        };
        let entries = list_entries_with(&root, &opts)?;
        assert_eq!(entries.len(), 3);
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn dir_stats_not_a_directory() {
        let err = dir_stats_with(Path::new("missing_dir_info_x"), 1).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::NotADirectory);
    }

    #[test]
    fn format_size_units() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(1023), "1023B");
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(1536), "1.5K");
        assert_eq!(format_size(1048576), "1.0M");
        assert_eq!(format_size(1572864), "1.5M");
        assert_eq!(format_size(1073741824), "1.0G");
    }
}
