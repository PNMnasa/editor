use std::{fs, io, path::PathBuf};

use editor_91to9::dir_info::{Entry, list_basic};
use editor_91to9::gui::{KeyCommand, step_selection, visible_indices};

fn entry(name: &str, is_dir: bool) -> Entry {
    Entry {
        name: name.to_owned(),
        is_dir,
        size: 0,
        files: 0,
        dirs: 0,
    }
}

#[test]
fn step_selection_clamps_to_visible_range() {
    assert_eq!(step_selection(0, 0, 10, KeyCommand::Down), 0);
    assert_eq!(step_selection(0, 0, 10, KeyCommand::Up), 0);
    assert_eq!(step_selection(3, 5, 10, KeyCommand::Up), 2);
    assert_eq!(step_selection(4, 5, 10, KeyCommand::Down), 4);
    assert_eq!(step_selection(0, 5, 10, KeyCommand::Home), 0);
    assert_eq!(step_selection(4, 5, 10, KeyCommand::End), 4);
}

#[test]
fn step_selection_pages_by_screen_height() {
    assert_eq!(step_selection(0, 100, 20, KeyCommand::PageDown), 20);
    assert_eq!(step_selection(30, 100, 20, KeyCommand::PageUp), 10);
    assert_eq!(step_selection(95, 100, 20, KeyCommand::PageDown), 99);
    assert_eq!(step_selection(5, 100, 20, KeyCommand::PageUp), 0);
}

#[test]
fn step_selection_area_rebounds_at_zero() {
    assert_eq!(step_selection(2, 5, 0, KeyCommand::PageUp), 1);
    assert_eq!(step_selection(2, 5, 1, KeyCommand::PageUp), 1);
}

#[test]
fn visible_indices_toggle_hidden() {
    let entries = [
        entry("a.txt", false),
        entry(".b", false),
        entry("dir", true),
    ];
    assert_eq!(visible_indices(&entries, false, ""), vec![0, 2]);
    assert_eq!(visible_indices(&entries, true, ""), vec![0, 1, 2]);
}

#[test]
fn visible_indices_filter_is_case_insensitive_substring() {
    let entries = [
        entry("README.md", false),
        entry("src", true),
        entry("readme.txt", false),
    ];
    assert_eq!(visible_indices(&entries, false, "read"), vec![0, 2]);
    assert_eq!(visible_indices(&entries, false, "SRC"), vec![1]);
    assert_eq!(visible_indices(&entries, false, "zzz"), Vec::<usize>::new());
}

#[test]
fn visible_indices_read_from_real_listing() -> io::Result<()> {
    let root = temp_root("gui")?;
    fs::write(root.join("a.txt"), b"a")?;
    fs::create_dir_all(root.join("sub"))?;
    fs::write(root.join(".hidden"), b"h")?;
    let entries = list_basic(&root)?;
    assert_eq!(visible_indices(&entries, false, ""), vec![0, 2]);
    assert_eq!(visible_indices(&entries, true, ""), vec![0, 1, 2]);
    assert_eq!(visible_indices(&entries, true, "txt"), vec![2]);
    fs::remove_dir_all(&root)?;
    Ok(())
}

fn temp_root(name: &str) -> io::Result<PathBuf> {
    let root = std::env::temp_dir().join(format!("editor_gui_{}_{}", std::process::id(), name));
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    fs::create_dir_all(&root)?;
    Ok(root)
}
