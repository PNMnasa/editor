//! High-level terminal UI primitives built on ANSI escape sequences.
//!
//! Lớp tiện ích UI bậc cao cho terminal, xây trên ANSI escape sequence.
//! Màu sắc dùng SGR; vẽ khung/dòng dùng ký tự box-drawing (UTF-8).
//!
//! Knowledge sources / Nguồn kiến thức để kiểm tra lại:
//! - ECMA-48 (ISO 6429) standard: <https://ecma-international.org/publications-and-standards/standards/ecma-48/>
//! - ANSI escape code (Wikipedia): <https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters>
//! - XTerm control sequences: <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html>
//! - Box-drawing characters (Unicode): <https://en.wikipedia.org/wiki/Box-drawing_character>

use std::io::{self, Write};

/// Reset foreground and background to the terminal defaults.
///
/// Đưa màu chữ và màu nền về mặc định của terminal (không reset thuộc tính khác).
/// Sequence: `CSI 39 m` (default foreground) + `CSI 49 m` (default background) — SGR.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters>
#[inline]
pub fn clear_color(out: &mut dyn Write) -> io::Result<()> {
    out.write_all(b"\x1b[39;49m")
}

/// Set the foreground to a basic SGR color code (30-37, 90-97).
///
/// Đặt màu chữ theo mã màu cơ bản SGR (30-37, 90-97).
/// Sequence: `CSI code m` (SGR) — ECMA-48.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters>
#[inline]
pub fn fg_color(out: &mut dyn Write, code: u8) -> io::Result<()> {
    write!(out, "\x1b[{code}m")
}

/// Set the background to a basic SGR color code (40-47, 100-107).
///
/// Đặt màu nền theo mã màu cơ bản SGR (40-47, 100-107).
/// Sequence: `CSI code m` (SGR) — ECMA-48.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters>
#[inline]
pub fn bg_color(out: &mut dyn Write, code: u8) -> io::Result<()> {
    write!(out, "\x1b[{code}m")
}

/// Set the foreground to an 8-bit (256-color) palette index.
///
/// Đặt màu chữ theo chỉ số bảng màu 8-bit (256 màu).
/// Sequence: `CSI 38;5;n m` (SGR extended color) — XTerm.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit_and_24-bit_color>
#[inline]
pub fn fg_indexed(out: &mut dyn Write, index: u8) -> io::Result<()> {
    write!(out, "\x1b[38;5;{index}m")
}

/// Set the background to an 8-bit (256-color) palette index.
///
/// Đặt màu nền theo chỉ số bảng màu 8-bit (256 màu).
/// Sequence: `CSI 48;5;n m` (SGR extended color) — XTerm.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit_and_24-bit_color>
#[inline]
pub fn bg_indexed(out: &mut dyn Write, index: u8) -> io::Result<()> {
    write!(out, "\x1b[48;5;{index}m")
}

/// Set the foreground to a 24-bit RGB color.
///
/// Đặt màu chữ theo màu 24-bit (truecolor).
/// Sequence: `CSI 38;2;r;g;b m` (SGR extended color) — XTerm.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit_and_24-bit_color>
#[inline]
pub fn fg_rgb(out: &mut dyn Write, r: u8, g: u8, b: u8) -> io::Result<()> {
    write!(out, "\x1b[38;2;{r};{g};{b}m")
}

/// Set the background to a 24-bit RGB color.
///
/// Đặt màu nền theo màu 24-bit (truecolor).
/// Sequence: `CSI 48;2;r;g;b m` (SGR extended color) — XTerm.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit_and_24-bit_color>
#[inline]
pub fn bg_rgb(out: &mut dyn Write, r: u8, g: u8, b: u8) -> io::Result<()> {
    write!(out, "\x1b[48;2;{r};{g};{b}m")
}

/// Write text starting at the given row and column.
///
/// Ghi chuỗi `text` bắt đầu tại dòng `row`, cột `col`.
/// Uses CUP to move, then writes the plain text.
///
/// Dùng CUP để di chuyển con trỏ, sau đó ghi text thường.
/// Source: <https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_(Control_Sequence_Introducer)_sequences>
#[inline]
pub fn put_text<S: AsRef<str>>(out: &mut dyn Write, row: u16, col: u16, text: S) -> io::Result<()> {
    write!(out, "\x1b[{row};{col}H{}", text.as_ref())
}

/// Draw a horizontal box-drawing line of `width` characters.
///
/// Vẽ đường kẻ ngang bằng ký tự box-drawing, dài `width` ký tự.
/// Uses CUP then repeats `─` (U+2500) `width` times.
///
/// Dùng CUP rồi lặp lại ký tự `─` (U+2500) `width` lần.
/// Source: <https://en.wikipedia.org/wiki/Box-drawing_character>
#[inline]
pub fn hline(out: &mut dyn Write, row: u16, col: u16, width: u16) -> io::Result<()> {
    write!(out, "\x1b[{row};{col}H")?;
    out.write_all("─".repeat(width as usize).as_bytes())
}

/// Draw a vertical box-drawing line of `height` characters.
///
/// Vẽ đường kẻ dọc bằng ký tự box-drawing, dài `height` ký tự.
/// Uses CUP per cell then writes `│` (U+2502) `height` times.
///
/// Dùng CUP tại từng vị trí rồi ghi ký tự `│` (U+2502) `height` lần.
/// Source: <https://en.wikipedia.org/wiki/Box-drawing_character>
#[inline]
pub fn vline(out: &mut dyn Write, row: u16, col: u16, height: u16) -> io::Result<()> {
    for r in row..row + height {
        write!(out, "\x1b[{r};{col}H│")?;
    }
    Ok(())
}

/// Draw a rectangle border starting at (row, col) with the given size.
///
/// Vẽ khung hình chữ nhật tại dòng `row`, cột `col`, kích thước `width` x `height`.
/// Corners use `┌┐└┘` (U+250C/2510/2514/2518); a size smaller than 2x2 draws nothing.
///
/// Góc dùng `┌┐└┘` (U+250C/2510/2514/2518); kích thước nhỏ hơn 2x2 sẽ không vẽ gì.
/// Source: <https://en.wikipedia.org/wiki/Box-drawing_character>
#[inline]
pub fn draw_box(
    out: &mut dyn Write,
    row: u16,
    col: u16,
    width: u16,
    height: u16,
) -> io::Result<()> {
    if width < 2 || height < 2 {
        return Ok(());
    }
    let filler = "─".repeat((width - 2) as usize);
    write!(out, "\x1b[{row};{col}H┌{filler}┐")?;
    for r in row + 1..row + height - 1 {
        write!(out, "\x1b[{r};{col}H│")?;
        write!(out, "\x1b[{r};{}H│", col + width - 1)?;
    }
    write!(out, "\x1b[{};{col}H└{filler}┘", row + height - 1)
}
