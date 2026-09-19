//! Thư viện hỗ trợ trình quản lý thư mục: liệt kê file/folder của một thư
//! mục kèm thông tin dung lượng, số lượng file/folder đệ quy và tổng kích
//! thước. Module thuần logic (không phụ thuộc terminal/UI) nên có thể dùng
//! riêng cho cả TUI lẫn GUI.
//!
//! Với thư mục lớn, dùng `list_basic` để vẽ danh sách ngay rồi chạy
//! `list_entries` (hoặc `dir_stats`) ở luồng nền để điền kích thước sau —
//! tránh làm delay giao diện.

use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
};

/// Số mục tối đa mặc định khi liệt kê một cấp (0 = không giới hạn).
pub const DEFAULT_MAX_ENTRIES: usize = 1000;
/// Độ sâu thư mục con tối đa mặc định khi tính thống kê (0 = không đi sâu).
pub const DEFAULT_MAX_DEPTH: usize = 16;

/// Tùy chọn quét giới hạn khối lượng công việc cho thư mục lớn/sâu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanOptions {
    /// Số mục tối đa liệt kê ở mỗi cấp; `0` = không giới hạn.
    pub max_entries: usize,
    /// Số cấp thư mục con được đi vào khi tính thống kê; `0` = chỉ mục trực tiếp.
    pub max_depth: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_ENTRIES,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

/// Thống kê cây thư mục: số file, số folder và tổng kích thước (đệ quy).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirStats {
    pub files: u64,
    pub dirs: u64,
    pub total_size: u64,
}

/// Một mục trong danh sách thư mục.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    /// Kích thước của file; với folder là tổng kích thước đệ quy.
    pub size: u64,
    /// Số file đệ quy (folder mới có).
    pub files: u64,
    /// Số folder đệ quy (folder mới có).
    pub dirs: u64,
}

/// Liệt kê nhanh nội dung `dir`: không tính thống kê đệ quy (folder được trả
/// về với `size/files/dirs = 0`). Dùng để vẽ danh sách trước, sau đó tính
/// kích thước ở luồng nền rồi ghi đè.
pub fn list_basic(dir: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = collect_basic(dir, &ScanOptions::default())?;
    sort_entries(&mut entries);
    Ok(entries)
}

/// Liệt kê nội dung `dir` kèm thống kê đệ quy cho từng folder, dùng tùy
/// chọn mặc định.
pub fn list_entries(dir: &Path) -> io::Result<Vec<Entry>> {
    list_entries_with(dir, &ScanOptions::default())
}

/// Liệt kê nội dung `dir` kèm thống kê đệ quy cho từng folder theo `opts`.
///
/// Kết quả được sắp xếp folder trước (theo tên không phân biệt hoa thường),
/// sau đó mới tới file. Mục không đọc được sẽ bị bỏ qua; lỗi của `dir` được
/// trả về nguyên vẹn. Nên dùng `list_basic` + luồng nền cho folder lớn.
pub fn list_entries_with(dir: &Path, opts: &ScanOptions) -> io::Result<Vec<Entry>> {
    let mut entries = collect_basic(dir, opts)?;
    if opts.max_depth > 0 {
        for entry in &mut entries {
            if entry.is_dir {
                if let Ok(stats) = dir_stats_at(&dir.join(&entry.name), opts.max_depth - 1) {
                    entry.size = stats.total_size;
                    entry.files = stats.files;
                    entry.dirs = stats.dirs;
                }
            }
        }
    }
    sort_entries(&mut entries);
    Ok(entries)
}

fn collect_basic(dir: &Path, opts: &ScanOptions) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for item in fs::read_dir(dir)? {
        if opts.max_entries != 0 && entries.len() >= opts.max_entries {
            break;
        }
        let Ok(item) = item else {
            continue;
        };
        let name = item.file_name().to_string_lossy().into_owned();
        let Ok(file_type) = item.file_type() else {
            continue;
        };
        let is_dir = file_type.is_dir();
        let size = if is_dir {
            0
        } else {
            item.metadata().map(|meta| meta.len()).unwrap_or(0)
        };
        entries.push(Entry {
            name,
            is_dir,
            size,
            files: 0,
            dirs: 0,
        });
    }
    Ok(entries)
}

fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Tính thống kê đệ quy của `dir` với độ sâu mặc định.
pub fn dir_stats(dir: &Path) -> io::Result<DirStats> {
    dir_stats_with(dir, DEFAULT_MAX_DEPTH)
}

/// Tính thống kê đệ quy của `dir`, đi sâu tối đa `max_depth` cấp.
///
/// `max_depth = 0` chỉ đếm mục trực tiếp của `dir` (không đi vào folder con).
/// Đi sâu quá giới hạn sẽ tính tới đó rồi dừng; folder sâu hơn không được cộng
/// thêm. Folder con không đọc được sẽ bị bỏ qua.
pub fn dir_stats_with(dir: &Path, max_depth: usize) -> io::Result<DirStats> {
    if !dir.is_dir() {
        return Err(io::Error::new(
            ErrorKind::NotADirectory,
            format!("{} không phải thư mục", dir.display()),
        ));
    }
    dir_stats_at(dir, max_depth)
}

fn dir_stats_at(dir: &Path, depth: usize) -> io::Result<DirStats> {
    let mut stats = DirStats::default();
    for item in fs::read_dir(dir)? {
        let Ok(item) = item else {
            continue;
        };
        let Ok(file_type) = item.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            if let Ok(meta) = item.metadata() {
                stats.files += 1;
                stats.total_size += meta.len();
            }
            continue;
        }
        stats.dirs += 1;
        if depth > 0 {
            if let Ok(sub) = dir_stats_at(&item.path(), depth - 1) {
                stats.files += sub.files;
                stats.dirs += sub.dirs;
                stats.total_size += sub.total_size;
            }
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format_tools::format_size;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> io::Result<PathBuf> {
        let root =
            std::env::temp_dir().join(format!("editor_dir_info_{}_{}", std::process::id(), name));
        if root.exists() {
            fs::remove_dir_all(&root)?;
        }
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    /// Cây mẫu:
    /// ```
    /// root/
    ///   dir1/
    ///     f1.txt (10B)
    ///     sub/
    ///       f2.txt (7B)
    ///   dir2/        (rỗng)
    ///   a.txt        (3B)
    /// ```
    fn make_tree(root: &Path) -> io::Result<()> {
        fs::create_dir_all(root.join("dir1/sub"))?;
        fs::create_dir_all(root.join("dir2"))?;
        fs::write(root.join("dir1/f1.txt"), vec![b'x'; 10])?;
        fs::write(root.join("dir1/sub/f2.txt"), vec![b'x'; 7])?;
        fs::write(root.join("a.txt"), vec![b'y'; 3])?;
        Ok(())
    }

    #[test]
    fn list_basic_no_stats() -> io::Result<()> {
        let root = temp_root("list_basic")?;
        make_tree(&root)?;
        let entries = list_basic(&root)?;
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["dir1", "dir2", "a.txt"]);
        assert!(
            entries
                .iter()
                .filter(|e| e.is_dir)
                .all(|e| e.size == 0 && e.files == 0 && e.dirs == 0)
        );
        assert_eq!(entries.last().map(|e| e.size), Some(3));
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn list_sorted_dirs_first_and_stats() -> io::Result<()> {
        let root = temp_root("list_sorted")?;
        make_tree(&root)?;
        let entries = list_entries(&root)?;
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["dir1", "dir2", "a.txt"]);

        let dir1 = &entries[0];
        assert!(dir1.is_dir);
        assert_eq!(dir1.files, 2);
        assert_eq!(dir1.dirs, 1);
        assert_eq!(dir1.size, 17);

        let dir2 = &entries[1];
        assert_eq!((dir2.files, dir2.dirs, dir2.size), (0, 0, 0));

        let file = &entries[2];
        assert!(!file.is_dir);
        assert_eq!((file.files, file.dirs, file.size), (0, 0, 3));
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn dir_stats_full() -> io::Result<()> {
        let root = temp_root("stats_full")?;
        make_tree(&root)?;
        let stats = dir_stats(&root)?;
        assert_eq!(stats.files, 3);
        assert_eq!(stats.dirs, 3);
        assert_eq!(stats.total_size, 20);
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn dir_stats_depth_limits() -> io::Result<()> {
        let root = temp_root("stats_depth")?;
        make_tree(&root)?;
        let depth0 = dir_stats_with(&root, 0)?;
        assert_eq!((depth0.files, depth0.dirs, depth0.total_size), (1, 2, 3));
        let depth1 = dir_stats_with(&root, 1)?;
        assert_eq!((depth1.files, depth1.dirs, depth1.total_size), (2, 3, 13));
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn max_entries_caps_listing() -> io::Result<()> {
        let root = temp_root("max_entries")?;
        for name in ["a.txt", "b.txt", "c.txt"] {
            fs::write(root.join(name), vec![b'x'; 4])?;
        }
        let opts = ScanOptions {
            max_entries: 2,
            max_depth: 1,
        };
        let entries = list_entries_with(&root, &opts)?;
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| !e.is_dir));
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn max_entries_zero_is_unlimited() -> io::Result<()> {
        let root = temp_root("max_entries_zero")?;
        make_tree(&root)?;
        let opts = ScanOptions {
            max_entries: 0,
            max_depth: 1,
        };
        let entries = list_entries_with(&root, &opts)?;
        assert_eq!(entries.len(), 3);
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn dir_stats_not_a_directory() {
        let err = dir_stats_with(Path::new("không_tồn_tại_dir_info_x"), 1).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::NotADirectory);
    }

    #[test]
    fn format_size_units() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(1023), "1023B");
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(1536), "1.5K");
        assert_eq!(format_size(1048576), "1.0M");
        assert_eq!(format_size(1572864), "1.5M");
        assert_eq!(format_size(1073741824), "1.0G");
    }
}
