//! Formatting helpers

/// Display width of `c` in monospace terminal columns: 0 for control
/// characters and combining marks, 1 for narrow glyphs, 2 for East-Asian
/// wide/fullwidth and emoji. A pragmatic approximation of the Unicode East
/// Asian Width property (no external table).
fn char_width(c: char) -> usize {
    let code = c as u32;
    if c.is_control() {
        return 0;
    }
    if matches!(
        code,
        0x0300..=0x036F
            | 0x0483..=0x0489
            | 0x0591..=0x05BD
            | 0x05BF
            | 0x05C1..=0x05C2
            | 0x05C4..=0x05C5
            | 0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x06D6..=0x06DC
            | 0x06DF..=0x06E4
            | 0x1AB0..=0x1AFF
            | 0x1DC0..=0x1DFF
            | 0x20D0..=0x20FF
            | 0xFE20..=0xFE2F
    ) {
        return 0;
    }
    if matches!(
        code,
        0x1100..=0x115F
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x2FFFD
    ) {
        return 2;
    }
    1
}

/// Clip `text` to at most `max` terminal columns for display.
///
/// `max = 0` yields the empty string. Longer text keeps its tail — the head
/// is dropped, and for `max >= 3` it is replaced with `...` so the end stays
/// readable. Text is truncated by display width, not character count: CJK/wide
/// glyphs and emoji count as two columns, combining marks as none.
pub fn clip(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.iter().map(|&c| char_width(c)).sum::<usize>() <= max {
        return text.to_owned();
    }
    let budget = if max < 3 { max } else { max - 3 };
    let mut tail: Vec<char> = Vec::new();
    let mut width = 0usize;
    for &c in chars.iter().rev() {
        let w = char_width(c);
        if width + w > budget {
            break;
        }
        width += w;
        tail.push(c);
    }
    let mut out: String = tail.into_iter().rev().collect();
    if max >= 3 {
        out.insert_str(0, "...");
    }
    out
}

/// Format a byte count into a readable unit (B..E, base 1024).
/// The largest `u64` (≈ 16 EiB) only reaches exabytes; ZB/YB need wider types.
///
/// The unit is chosen by counting the most significant set bit
/// (`leading_zeros` — a single CPU instruction, no loop): dividing that bit
/// position by 10 yields the power of 1024.
pub fn format_size(bytes: u64) -> String {
    const UNITS: [char; 7] = ['B', 'K', 'M', 'G', 'T', 'P', 'E'];
    let bits = u64::BITS - bytes.leading_zeros();
    let scale_index = bits.saturating_sub(1) / 10;
    if scale_index == 0 {
        return format!("{bytes}B");
    }
    let shift = scale_index * 10;
    let scale = 1u64 << shift;
    let frac = ((bytes & (scale - 1)) * 10 + scale / 2) >> shift;
    format!(
        "{}.{}{}",
        bytes / scale + frac / 10,
        frac % 10,
        UNITS[scale_index as usize]
    )
}
