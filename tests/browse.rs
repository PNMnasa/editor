use std::{fs, io, path::PathBuf, thread, time::Duration};

use editor_91to9::browse::{Browser, visible_indices};
use editor_91to9::dir_info::{Entry, list_basic};

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
    let root = temp_root("visible")?;
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

#[test]
fn browser_navigate_lists_immediately_then_enriches() -> io::Result<()> {
    let root = temp_root("navigate")?;
    fs::write(root.join("a.txt"), b"a")?;
    fs::create_dir_all(root.join("sub"))?;
    fs::write(root.join("sub/b.bin"), b"bb")?;

    let mut browser = Browser::new();
    assert!(browser.navigate(&root));
    assert!(browser.is_computing());
    assert_eq!(browser.message(), "Computing sizes…");
    assert_eq!(browser.dir(), root.as_path());
    assert_eq!(browser.entries(), list_basic(&root)?.as_slice());

    wait_until_done(&mut browser);
    assert!(!browser.is_computing());
    assert_eq!(browser.message(), "");
    let sub = browser
        .entries()
        .iter()
        .find(|e| e.name == "sub")
        .expect("sub dir listed");
    assert_eq!((sub.files, sub.dirs, sub.size), (1, 0, 2));

    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn browser_navigate_missing_dir_fails() {
    let missing =
        std::env::temp_dir().join(format!("editor_browse_missing_{}", std::process::id()));
    let mut browser = Browser::new();
    assert!(!browser.navigate(&missing));
    assert!(!browser.is_computing());
    assert!(browser.message().contains("Cannot read"));
}

#[test]
fn browser_set_message_is_overwritten_by_navigation() -> io::Result<()> {
    let root = temp_root("message")?;
    let mut browser = Browser::new();
    browser.set_message("opening is not supported");
    assert_eq!(browser.message(), "opening is not supported");
    assert!(browser.navigate(&root));
    assert_eq!(browser.message(), "Computing sizes…");
    fs::remove_dir_all(&root)?;
    Ok(())
}

fn wait_until_done(browser: &mut Browser) {
    for _ in 0..500 {
        browser.poll();
        if !browser.is_computing() {
            return;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("background scan did not finish in time");
}

fn temp_root(name: &str) -> io::Result<PathBuf> {
    let root = std::env::temp_dir().join(format!("editor_browse_{}_{}", std::process::id(), name));
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    fs::create_dir_all(&root)?;
    Ok(root)
}
