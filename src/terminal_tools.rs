//! Direct terminal control via ANSI escape sequences.
//!
//! Knowledge sources:
//! - ECMA-48 (ISO 6429) standard: <https://ecma-international.org/publications-and-standards/standards/ecma-48/>
//! - XTerm control sequences: <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html>
//! - ANSI escape code (Wikipedia): <https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_(Control_Sequence_Introducer)_sequences>
//! - VT100 User Guide: <https://vt100.net/docs/vt100-ug/chapter3.html>

use std::io::{self, Write};

/// Move the cursor to the given row and column (1-based).
///
/// Sequence: `CSI Pl;Pc H` (CUP, Cursor Position) — ECMA-48.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_(Control_Sequence_Introducer)_sequences>
#[inline]
pub fn goto(out: &mut dyn Write, row: u16, col: u16) -> io::Result<()> {
    write!(out, "\x1b[{row};{col}H")
}

/// Clear the whole screen and move the cursor home.
///
/// Sequences: `CSI 2J` (ED, Erase in Display) + `CSI H` (CUP home) — ECMA-48.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_(Control_Sequence_Introducer)_sequences>
#[inline]
pub fn clear(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"\x1b[2J\x1b[H")
}

/// Clear the whole line the cursor is on.
///
/// Sequence: `CSI 2K` (EL, Erase in Line) — ECMA-48.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_(Control_Sequence_Introducer)_sequences>
#[inline]
pub fn clear_line(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"\x1b[2K")
}

/// Show the cursor.
///
/// Sequence: `CSI ?25h` (DECTCEM show) — DEC private mode.
/// Source: <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-PC-Style-Mouse-Tracking>
#[inline]
pub fn show_cursor(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"\x1b[?25h")
}

/// Hide the cursor, usually before entering a TUI.
///
/// Sequence: `CSI ?25l` (DECTCEM hide) — DEC private mode.
/// Source: <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-PC-Style-Mouse-Tracking>
#[inline]
pub fn hide_cursor(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"\x1b[?25l")
}

/// Save the current cursor position.
///
/// Sequence: `ESC 7` (DECSC, Save Cursor) — VT100.
/// Source: <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Controls-beginning-with-ESC>
#[inline]
pub fn save_cursor(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"\x1b7")
}

/// Restore the previously saved cursor position.
///
/// Sequence: `ESC 8` (DECRC, Restore Cursor) — VT100.
/// Source: <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Controls-beginning-with-ESC>
#[inline]
pub fn restore_cursor(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"\x1b8")
}

/// Enter the alternate screen buffer, keeping the normal screen intact.
///
/// Sequence: `CSI ?1049h` (use alternate screen buffer) — DEC private mode.
/// Source: <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Operating-System-Commands>
#[inline]
pub fn enter_alt_screen(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"\x1b[?1049h")
}

/// Leave the alternate screen buffer and return to the normal screen.
///
/// Sequence: `CSI ?1049l` (use normal screen buffer) — DEC private mode.
/// Source: <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Operating-System-Commands>
#[inline]
pub fn leave_alt_screen(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"\x1b[?1049l")
}

/// Set the terminal window title.
///
/// Sequence: `OSC 0 ; title BEL` (window icon/name) — XTerm.
/// Source: <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Operating-System-Commands>
#[inline]
pub fn set_title<S: AsRef<str>>(out: &mut dyn Write, title: S) -> io::Result<()> {
    write!(out, "\x1b]0;{}\x07", title.as_ref())
}

/// Reset all text attributes to their defaults.
///
/// Sequence: `CSI 0m` (SGR, Select Graphic Rendition) — ECMA-48.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters>
#[inline]
pub fn reset_style(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"\x1b[0m")
}
