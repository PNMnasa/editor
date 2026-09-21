//! Formatting helpers

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

#[cfg(test)]
mod tests {
    use super::format_size;

    const KB: u64 = 1 << 10;
    const MB: u64 = 1 << 20;
    const GB: u64 = 1 << 30;
    const TB: u64 = 1 << 40;
    const PB: u64 = 1 << 50;
    const EB: u64 = 1 << 60;

    const UNITS: [char; 7] = ['B', 'K', 'M', 'G', 'T', 'P', 'E'];

    /// Reference implementation (naive, easy to reason about as correct): the
    /// unit is chosen by multiplying `scale` incrementally, then integer
    /// division plus one decimal digit, rounded. Like `format_size` but
    /// without `leading_zeros`.
    fn reference(bytes: u64) -> String {
        let mut scale: u64 = 1;
        let mut unit: usize = 0;
        while unit + 1 < UNITS.len() {
            let Some(next) = scale.checked_mul(1024) else {
                break;
            };
            if next > bytes {
                break;
            }
            scale = next;
            unit += 1;
        }
        if unit == 0 {
            return format!("{bytes}B");
        }
        let frac = ((bytes % scale) * 10 + scale / 2) / scale;
        format!("{}.{}{}", bytes / scale + frac / 10, frac % 10, UNITS[unit])
    }

    /// `1024^k` (None when `k` is so large it would overflow `u64`).
    fn pow1024(k: u32) -> Option<u64> {
        let mut value = 1u64;
        for _ in 0..k {
            value = value.checked_mul(1024)?;
        }
        Some(value)
    }

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

    #[test]
    fn known_values() {
        let cases: &[(u64, &str)] = &[
            (0, "0B"),
            (1, "1B"),
            (1023, "1023B"),
            (1024, "1.0K"),
            (1536, "1.5K"),
            (MB, "1.0M"),
            (1572864, "1.5M"),
            (GB, "1.0G"),
            (TB, "1.0T"),
            (PB, "1.0P"),
            (EB, "1.0E"),
            (EB + EB / 2, "1.5E"),
            (u64::MAX, "16.0E"),
        ];
        for &(bytes, expected) in cases {
            assert_eq!(format_size(bytes), expected, "bytes = {bytes}");
        }
    }

    #[test]
    fn matches_reference_on_dense_range() {
        for bytes in 0..=1_000_000u64 {
            assert_eq!(format_size(bytes), reference(bytes), "bytes = {bytes}");
        }
    }

    #[test]
    fn matches_reference_on_powers_and_boundaries() {
        let mut cases = Vec::new();
        for bit in 0..=63u32 {
            let value = 1u64 << bit;
            cases.push(value);
            cases.push(value - 1);
            cases.push(value + 1);
        }
        for k in 0..=7u32 {
            if let Some(value) = pow1024(k) {
                cases.push(value.saturating_sub(1));
                cases.push(value);
                cases.push(value + 1);
                cases.push(value + value / 2);
            }
        }
        cases.push(u64::MAX - 1);
        cases.push(u64::MAX);
        for bytes in cases {
            assert_eq!(format_size(bytes), reference(bytes), "bytes = {bytes}");
        }
    }

    #[test]
    fn matches_reference_on_random_sweep() {
        // Simple LCG with deterministic results — sweeps the whole `u64`
        // domain (including near 0).
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        for _ in 0..100_000 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1_442_695_040_888_963_407);
            assert_eq!(format_size(state), reference(state), "bytes = {state}");
        }
    }

    #[test]
    fn never_overflows() {
        // Upper bound of the fractional math: `rest*10 + scale/2` must match
        // the ceiling via `reference` — an overflow would break `reference`
        // too, so the cross-checks at u64::MAX catch it.
        assert_eq!(format_size(u64::MAX), reference(u64::MAX));
        assert_eq!(format_size(u64::MAX - 1), reference(u64::MAX - 1));
    }
}
