//! Formatting helpers

/// Format a byte count into a readable unit (B, K, M, G — base 1024).
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1 << 10;
    const MB: u64 = 1 << 20;
    const GB: u64 = 1 << 30;
    if bytes >= GB {
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
