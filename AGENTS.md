# AGENTS.md

Hướng dẫn cho agent khi làm việc trong dự án `editor`.

## Bối cảnh

- Đọc `README.md` để nắm mục tiêu dự án, `docs/INSTALL.md` cho cách build/cài đặt.
- Crate Rust đơn (`editor`), edition 2024, MSRV 1.85, dependency duy nhất `crossterm`.
- Mục tiêu TUI + GUI, giấy phép GPL-3.0 — phần đóng góp phải tương thích.

## Lệnh thường dùng

- Build: `cargo build`; bản release: `cargo build --release`
- Kiểm tra biên dịch: `cargo check`
- Chạy test: `cargo test`
- Chạy một test cụ thể: `cargo test dir_info::tests::tên_hàm`
- Format: `cargo fmt --check` (kiểm tra), `cargo fmt` (sửa)
- Lint: `cargo clippy --all-targets -- -D warnings`
- Chạy chương trình: `cargo run`

## Cấu trúc & gotcha

- 4 file trong `src/`:
  - `main.rs` — binary entry, explorer TUI.
  - `dir_info.rs` — logic liệt kê file/folder, thống kê kích thước; **chứa tất cả test** của dự án (`#[cfg(test)] mod tests`).
  - `terminal_tools.rs` — ANSI helpers.
  - `terminal_ui_tools.rs` — text/color drawing.
- Tất cả modules đều có `#[expect(dead_code)]` ở cấp `mod` trong `main.rs`. Nếu dùng toàn bộ public items từ một module, lint `unfulfilled_lint_expectations` sẽ kích hoạt và clippy fail `-D warnings`. Cần giữ hoặc bỏ `#[expect]` cho phù hợp.
- CI (`.github/workflows/ci.yml`): 4 bước — `fmt --check`, `clippy --all-targets -- -D warnings`, `test`, `build --release` — chạy trên Windows, Linux, macOS.
- Release workflow (`.github/workflows/release.yml`): trigger bởi tag `v*` trên `main`, build release 3 nền tảng và tạo GitHub release.

## Quy ước quan trọng

- **Không sửa `src/` trừ khi được yêu cầu trực tiếp** — mã nguồn đang ở giai đoạn giữ nguyên.
- **Thử nghiệm trên nhánh local, không push**: mỗi hướng thử nghiệm tạo nhánh local riêng (đặt tên có tiền tố như `dev/<tên>` hoặc `agents/<tên>`) từ `develop`; chỉ push nhánh lên GitHub khi được yêu cầu trực tiếp để remote luôn sạch.
- Mọi thay đổi phải pass đủ 4 bước CI: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`.
- Tuân thủ `CONTRIBUTING.md` khi đóng góp.
- Ghi thay đổi đáng chú ý vào `CHANGELOG.md` (đúng định dạng Keep a Changelog).
- Tài liệu viết tiếng Việt; mã nguồn, tên biến/hàm dùng tiếng Anh.

## Release nhanh

- Một lệnh cho toàn bộ pipeline: `.\scripts\release.ps1 "message commit"` — tự chạy 4 bước CI, commit sạch (hoặc tự nhóm theo area nếu không truyền message), push `develop`, merge `main`, tạo tag (tự tăng patch nếu không truyền `-Version`), push tag (kích hoạt release workflow), rồi quay lại `develop`.
- **Phải ở nhánh `develop`** — script sẽ dừng nếu không phải.
- Tùy chọn: `-SkipChecks` (bỏ qua CI bước); `-Version v0.2.0` (gán tag cụ thể); `-DryRun` (chỉ in thay đổi, không commit).