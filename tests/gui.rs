use editor_91to9::gui::{KeyCommand, step_selection};

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
