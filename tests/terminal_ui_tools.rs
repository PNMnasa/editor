use std::borrow::Cow;

use editor_91to9::terminal_ui_tools::{put_text, sanitize_for_terminal};

#[test]
fn clean_text_is_borrowed_unchanged() {
    let text = "plain name, with spaces 42";
    match sanitize_for_terminal(text) {
        Cow::Borrowed(clean) => assert_eq!(clean, text),
        Cow::Owned(_) => panic!("clean input must not allocate"),
    }
}

#[test]
fn c0_controls_use_caret_notation() {
    assert_eq!(sanitize_for_terminal("a\x1b\x07\x0ab"), "a^[^G^Jb");
    assert_eq!(sanitize_for_terminal("\x00"), "^@");
    assert_eq!(sanitize_for_terminal("\x1f"), "^_");
    assert_eq!(sanitize_for_terminal("\x7f"), "^?");
}

#[test]
fn c1_controls_become_replacement_char() {
    assert_eq!(sanitize_for_terminal("a\u{9f}b"), "a\u{fffd}b");
    assert_eq!(sanitize_for_terminal("\u{80}"), "\u{fffd}");
}

#[test]
fn put_text_strips_injection_but_keeps_csi_move() {
    let mut buf = Vec::new();
    put_text(&mut buf, 3, 5, "x\x1b[31m").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "\x1b[3;5Hx^[[31m");
}
