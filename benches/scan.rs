//! Dependency-free manual micro-benchmark for directory scanning and for
//! cross-checking `format_size` against alternative algorithms.
//! Every format variant is a zero-alloc `Display`: it writes directly into a
//! `fmt::Write`, never builds a `String` — measuring pure computation cost
//! without allocation overhead. Run with: `cargo bench --bench scan` or with
//! args: `cargo bench --bench scan 50 10` (50 scans, 10 format rounds).

//! Modules included via `#[path]` are compiled with `--cfg test` but WITHOUT a
//! test harness, so their `#[cfg(test)]` blocks are not "used" and get flagged
//! dead_code/unused_imports. That lint applies only to the bench file, not to
//! the main crate.

#[path = "../src/format_tools.rs"]
#[allow(dead_code, unused_imports)]
mod format_tools;

#[path = "../src/dir_info.rs"]
#[allow(dead_code, unused_imports)]
mod dir_info;

use std::{fs, path::Path, time::Instant};

use dir_info::{ScanOptions, list_entries_with};

const UNITS: [char; 7] = ['B', 'K', 'M', 'G', 'T', 'P', 'E'];
const KB: u64 = 1 << 10;
const MB: u64 = 1 << 20;
const GB: u64 = 1 << 30;
const TB: u64 = 1 << 40;
const PB: u64 = 1 << 50;
const EB: u64 = 1 << 60;

/// The Windows `StrFormatByteSizeW` (shlwapi) variant as a zero-alloc `Display`:
/// the API writes directly into a fixed `u16` buffer, `fmt` decodes UTF-16
/// into the formatter, no `String`. Note: it uses base 1000 rather than 1024,
/// and the output has locale-specific commas/normalization — speed only, the
/// output is not compared.
#[cfg(windows)]
mod win_native {
    use std::fmt;

    #[link(name = "shlwapi")]
    unsafe extern "system" {
        fn StrFormatByteSizeW(qw: u64, pszbuf: *mut u16, cchbuf: u32) -> *mut u16;
    }

    #[derive(Clone, Copy)]
    pub struct WinFormat(u64);

    impl WinFormat {
        pub fn new(bytes: u64) -> Self {
            Self(bytes)
        }
    }

    impl fmt::Display for WinFormat {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use std::fmt::Write;
            unsafe {
                let mut buf = [0u16; 64];
                StrFormatByteSizeW(self.0, buf.as_mut_ptr(), buf.len() as u32);
                let len = buf.iter().position(|&u| u == 0).unwrap_or(buf.len());
                for unit in char::decode_utf16(buf[..len].iter().copied()) {
                    f.write_char(unit.unwrap_or(char::REPLACEMENT_CHARACTER))?;
                }
            }
            Ok(())
        }
    }
}

use std::fmt::{self, Write};

/// The **zero-alloc** variants: `Display` writes directly into the formatter,
/// never builds a `String`. `format!("{}")` still allocates, but
/// `write!(dst, "{}")` with `dst: fmt::Write` does not. Each variant keeps the
/// unit-selection logic of its algorithm; the output is cross-checked against
/// `format_tools::format_size`.
#[derive(Clone, Copy)]
struct FormatSizeLeading(u64);

impl fmt::Display for FormatSizeLeading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0;
        let bits = u64::BITS - bytes.leading_zeros();
        let scale_index = (bits.saturating_sub(1) / 10) as usize;
        if scale_index == 0 {
            return write!(f, "{bytes}B");
        }
        let shift = scale_index * 10;
        let scale = 1u64 << shift;
        let frac = ((bytes & (scale - 1)) * 10 + scale / 2) >> shift;
        write!(
            f,
            "{}.{}{}",
            bytes / scale + frac / 10,
            frac % 10,
            UNITS[scale_index]
        )
    }
}

#[derive(Clone, Copy)]
struct FormatSizeIlog2(u64);

impl fmt::Display for FormatSizeIlog2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0;
        let scale_index = bytes.checked_ilog2().map_or(0, |log| log / 10) as usize;
        if scale_index == 0 {
            return write!(f, "{bytes}B");
        }
        let shift = scale_index * 10;
        let scale = 1u64 << shift;
        let frac = ((bytes & (scale - 1)) * 10 + scale / 2) >> shift;
        write!(
            f,
            "{}.{}{}",
            bytes / scale + frac / 10,
            frac % 10,
            UNITS[scale_index]
        )
    }
}

#[derive(Clone, Copy)]
struct FormatSizeChain(u64);

impl fmt::Display for FormatSizeChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0;
        if bytes >= EB {
            let value = ((bytes & (EB - 1)) * 10 + EB / 2) >> 60;
            write!(f, "{}.{}E", (bytes >> 60) + value / 10, value % 10)
        } else if bytes >= PB {
            let value = ((bytes & (PB - 1)) * 10 + PB / 2) >> 50;
            write!(f, "{}.{}P", (bytes >> 50) + value / 10, value % 10)
        } else if bytes >= TB {
            let value = ((bytes & (TB - 1)) * 10 + TB / 2) >> 40;
            write!(f, "{}.{}T", (bytes >> 40) + value / 10, value % 10)
        } else if bytes >= GB {
            let value = ((bytes & (GB - 1)) * 10 + GB / 2) >> 30;
            write!(f, "{}.{}G", (bytes >> 30) + value / 10, value % 10)
        } else if bytes >= MB {
            let value = ((bytes & (MB - 1)) * 10 + MB / 2) >> 20;
            write!(f, "{}.{}M", (bytes >> 20) + value / 10, value % 10)
        } else if bytes >= KB {
            let value = ((bytes & 1023) * 10 + 512) >> 10;
            write!(f, "{}.{}K", (bytes >> 10) + value / 10, value % 10)
        } else {
            write!(f, "{bytes}B")
        }
    }
}

#[derive(Clone, Copy)]
struct FormatSizeLoop(u64);

impl fmt::Display for FormatSizeLoop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0;
        let mut scale: u64 = 1;
        let mut unit = 0usize;
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
            return write!(f, "{bytes}B");
        }
        let frac = ((bytes % scale) * 10 + scale / 2) / scale;
        write!(
            f,
            "{}.{}{}",
            bytes / scale + frac / 10,
            frac % 10,
            UNITS[unit]
        )
    }
}

/// Sink that only counts written bytes — stores nothing, so format cost is
/// measured accurately.
struct Sink(u64);

impl fmt::Write for Sink {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 = self.0.wrapping_add(s.len() as u64);
        Ok(())
    }
}

/// Times one zero-alloc `Display` variant: if a `reference` is provided the
/// output is cross-checked before measuring; the measurement loop only
/// `write!`s straight into the sink, never building a `String`.
fn time_zero_alloc<D>(
    name: &str,
    make: impl Fn(u64) -> D,
    reference: Option<fn(u64) -> String>,
    sizes: &[u64],
    reps: usize,
) where
    D: fmt::Display + Copy,
{
    if let Some(reference) = reference {
        for &b in sizes {
            assert_eq!(format!("{}", make(b)), reference(b), "{name} at {b}");
        }
    }
    let mut sink = Sink(0);
    let first = make(sizes[0]);
    std::hint::black_box(write!(&mut sink, "{first}")).unwrap();
    std::hint::black_box(sink.0);
    let start = Instant::now();
    for _ in 0..reps {
        for &b in sizes {
            write!(&mut sink, "{}", make(b)).unwrap();
        }
    }
    std::hint::black_box(sink.0);
    let ns = start.elapsed().as_nanos() as f64;
    println!(
        "format(Display 0-alloc {name}): total {:?}, ~{:.2} ns/sample",
        start.elapsed(),
        ns / (sizes.len() * reps) as f64
    );
}

fn build_tree(root: &Path) {
    fn fill(dir: &Path, depth: usize) {
        for i in 0..4 {
            let sub = dir.join(format!("d{i}"));
            fs::create_dir_all(&sub).unwrap();
            for j in 0..16 {
                fs::write(sub.join(format!("f{j}.txt")), vec![b'x'; 32]).unwrap();
            }
            if depth > 1 {
                fill(&sub, depth - 1);
            }
        }
    }
    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root).unwrap();
    fill(root, 5);
}

/// A size distribution close to reality (many small files, rare large ones):
/// 0..4096 dense, every power of two, plus random samples heavily skewed to
/// the small units — B 40%, K 30%, M 20%, G 8%, T 1.5%, P 0.4%, E 0.1%.
fn format_samples() -> Vec<u64> {
    // Cumulative boundary of the 7 unit groups (B, K, M, G, T, P, E).
    const BUCKETS: [u64; 7] = [400, 700, 900, 980, 995, 999, 1001];

    struct Rng(u64);
    impl Rng {
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
    }

    let mut sizes = Vec::with_capacity(131_072);
    for b in 0..4096u64 {
        sizes.push(b);
    }
    for bit in 0..=63u32 {
        sizes.push(1u64 << bit);
    }
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    while sizes.len() < 131_072 {
        // Unit rank weighted to simulate the real distribution.
        let r = rng.next_u64() % 1001;
        let unit = BUCKETS.partition_point(|&top| r > top);
        let size = if unit == 0 {
            rng.next_u64() % 1024
        } else {
            // In [1024^unit, 1024^(unit+1)) — the value belongs to this
            // exact rank.
            let mut scale = 1u64;
            for _ in 0..unit {
                scale *= 1024;
            }
            scale + rng.next_u64() % scale
        };
        sizes.push(size);
    }
    assert_eq!(sizes.len(), 131_072);
    sizes
}

fn bench_formats(reps: usize) {
    let sizes = format_samples();
    let reference = Some(format_tools::format_size as fn(u64) -> String);
    println!(
        "cross-checking output against format_size on {} samples (0-alloc)...",
        sizes.len()
    );
    time_zero_alloc(
        "leading_zeros (current)",
        FormatSizeLeading,
        reference,
        &sizes,
        reps,
    );
    time_zero_alloc("u64::ilog2 (std)", FormatSizeIlog2, reference, &sizes, reps);
    time_zero_alloc("if/else chain", FormatSizeChain, reference, &sizes, reps);
    time_zero_alloc("multiply loop", FormatSizeLoop, reference, &sizes, reps);
    #[cfg(windows)]
    time_zero_alloc(
        "StrFormatByteSizeW (Windows)",
        win_native::WinFormat::new,
        None,
        &sizes,
        reps,
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scan_samples: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(20);
    let format_reps: usize = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(10);

    let root = std::env::temp_dir().join("editor_bench_scan");
    build_tree(&root);
    let opts = ScanOptions::default();
    println!("sample tree: {}", root.display());

    std::hint::black_box(list_entries_with(&root, &opts).unwrap());
    let start = Instant::now();
    for _ in 0..scan_samples {
        std::hint::black_box(list_entries_with(&root, &opts).unwrap());
    }
    let per = start.elapsed() / scan_samples as u32;
    println!(
        "scan {scan_samples} times: {:?} total, ~{:?}/each",
        start.elapsed(),
        per
    );

    let _ = fs::remove_dir_all(&root);

    bench_formats(format_reps);
}
