//! Formatting helpers

/// Format a byte count into a human-readable unit (B..E, base 1024).
/// The largest `u64` (≈ 16 EiB) only reaches exabytes; ZB/YB need wider numbers.
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1 << 10;
    const MB: u64 = 1 << 20;
    const GB: u64 = 1 << 30;
    const TB: u64 = 1 << 40;
    const PB: u64 = 1 << 50;
    const EB: u64 = 1 << 60;
    if bytes >= EB {
        let value = ((bytes & (EB - 1)) * 10 + EB / 2) >> 60;
        format!("{}.{}E", (bytes >> 60) + value / 10, value % 10)
    } else if bytes >= PB {
        let value = ((bytes & (PB - 1)) * 10 + PB / 2) >> 50;
        format!("{}.{}P", (bytes >> 50) + value / 10, value % 10)
    } else if bytes >= TB {
        let value = ((bytes & (TB - 1)) * 10 + TB / 2) >> 40;
        format!("{}.{}T", (bytes >> 40) + value / 10, value % 10)
    } else if bytes >= GB {
        let value = ((bytes & (GB - 1)) * 10 + GB / 2) >> 30;
        format!("{}.{}G", (bytes >> 30) + value / 10, value % 10)
    } else if bytes >= MB {
        let value = ((bytes & (MB - 1)) * 10 + MB / 2) >> 20;
        format!("{}.{}M", (bytes >> 20) + value / 10, value % 10)
    } else if bytes >= KB {
        let value = ((bytes & 1023) * 10 + 512) >> 10;
        format!("{}.{}K", (bytes >> 10) + value / 10, value % 10)
    } else {
        format!("{bytes}B")
    }
}

#[cfg(test)]
mod tests {
    use super::format_size;

    const KB: u64 = 1 << 10;
    const MB: u64 = 1 << 20;
    const GB: u64 = 1 << 30;
    const TB: u64 = 1 << 40;
    const PB: u64 = 1 << 50;
    const EB: u64 = 1 << 60;

    #[test]
    fn small_bytes_are_bare() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(1023), "1023B");
    }

    #[test]
    fn kilo_and_mega() {
        assert_eq!(format_size(KB), "1.0K");
        assert_eq!(format_size(KB + KB / 2), "1.5K");
        assert_eq!(format_size(MB), "1.0M");
        assert_eq!(format_size(MB + MB / 2), "1.5M");
    }

    #[test]
    fn giga_and_tera() {
        assert_eq!(format_size(GB), "1.0G");
        assert_eq!(format_size(TB), "1.0T");
        assert_eq!(format_size(TB + TB / 2), "1.5T");
        assert_eq!(format_size(TB * 2), "2.0T");
    }

    #[test]
    fn peta_and_exa() {
        assert_eq!(format_size(PB), "1.0P");
        assert_eq!(format_size(PB + PB / 2), "1.5P");
        assert_eq!(format_size(EB), "1.0E");
        assert_eq!(format_size(EB + EB / 2), "1.5E");
    }
}
