//! Formatting helpers

/// Clip `text` to at most `max` characters for display.
///
/// `max = 0` yields the empty string. Longer text keeps its tail — the head
/// is dropped, and for `max >= 3` it is replaced with `...` so the end stays
/// readable.
pub fn clip(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        text.to_owned()
    } else if max < 3 {
        chars[chars.len() - max..].iter().collect()
    } else {
        let mut out: String = chars[chars.len() - max + 3..].iter().collect();
        out.insert_str(0, "...");
        out
    }
}

/// Format a byte count into a readable unit (B..E, base 1024).
/// The largest `u64` (≈ 16 EiB) only reaches exabytes; ZB/YB need wider types.
///
/// The unit is chosen by counting the most significant set bit
/// (`leading_zeros` — a single CPU instruction, no loop): dividing that bit
/// position by 10 yields the power of 1024. The old `if/else` chain was
/// replaced by the equivalent bit math.
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
