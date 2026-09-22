use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use editor_91to9::dir_info::{
    ScanOptions, dir_stats, dir_stats_with, list_basic, list_entries, list_entries_with,
    list_entries_with_checked,
};

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
fn list_basic_no_stats() -> io::Result<()> {
    let root = temp_root("list_basic")?;
    make_tree(&root)?;
    let entries = list_basic(&root)?;
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["dir1", "dir2", "a.txt"]);
    assert!(
        entries
            .iter()
            .filter(|e| e.is_dir)
            .all(|e| e.size == 0 && e.files == 0 && e.dirs == 0)
    );
    assert_eq!(entries.last().map(|e| e.size), Some(3));
    fs::remove_dir_all(&root)?;
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
fn checked_scan_cancelled_immediately() -> io::Result<()> {
    let root = temp_root("cancel")?;
    make_tree(&root)?;
    let cancel = AtomicBool::new(true);
    let err = list_entries_with_checked(&root, &ScanOptions::default(), &cancel).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Interrupted);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn checked_scan_normally_completes() -> io::Result<()> {
    let root = temp_root("cancel_done")?;
    make_tree(&root)?;
    let cancel = AtomicBool::new(false);
    let entries = list_entries_with_checked(&root, &ScanOptions::default(), &cancel)?;
    assert_eq!(entries.len(), 3);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlinked_dir_counts_as_dir() -> io::Result<()> {
    let root = temp_root("symlink")?;
    make_tree(&root)?;
    std::os::unix::fs::symlink(root.join("dir1"), root.join("link"))?;
    let entries = list_entries(&root)?;
    let link = entries
        .iter()
        .find(|e| e.name == "link")
        .expect("symlink listed");
    assert!(link.is_dir);
    assert_eq!((link.files, link.dirs, link.size), (2, 1, 17));
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn dir_stats_not_a_directory() -> io::Result<()> {
    let missing =
        std::env::temp_dir().join(format!("editor_dir_info_missing_{}", std::process::id()));
    let _ = fs::remove_dir_all(&missing);
    let err = dir_stats_with(&missing, 1).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotADirectory);
    Ok(())
}
