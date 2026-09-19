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

- Các module trong `src/` (theo vai trò, không cố định số file — đang thêm dần):
  - `main.rs` — binary entry, explorer TUI; vẽ danh sách ngay bằng `list_basic`, tính kích thước ở luồng nền (`list_entries`) rồi ghi đè.
  - `dir_info.rs` — liệt kê file/folder, thống kê kích thước; **chứa tất cả test** của dự án (`#[cfg(test)] mod tests`). Test tự tạo temp dirs thật, không cần fixture/dịch vụ ngoài.
  - `format_tools.rs` — format số/chuỗi (hiện có `format_size`).
  - `terminal_tools.rs` — ANSI helpers.
  - `terminal_ui_tools.rs` — vẽ text/color/box.
- Tất cả modules đều có `#[expect(dead_code)]` ở cấp `mod` trong `main.rs`. Nếu dùng toàn bộ công khai từ một module, `#[expect]` đó trở thành `unfulfilled_lint_expectations` và clippy `-D warnings` sẽ fail — giữ hoặc bỏ `#[expect]` cho từng module.
- `opencode.json` (và `.opencode/agent/reviewer.md`) cấu hình OpenCode: chỉ `git *` và `cargo *` được chạy không cần hỏi; mọi lệnh shell khác sẽ hỏi lại user.
- Trong `.opencode/`, chỉ `agent/reviewer.md` được track; `package*.json` và `node_modules` là scratch của plugin, đã gitignore — đừng commit chúng.
- CI (`.github/workflows/ci.yml`): 4 bước — `fmt --check`, `clippy --all-targets -- -D warnings`, `test`, `build --release` — chạy trên Windows, Linux, macOS.
- Release workflow (`.github/workflows/release.yml`): trigger bởi tag `v*` trên `main`, build release 3 nền tảng và tạo GitHub release (binary đổi tên `editor-linux` / `editor-windows.exe` / `editor-macos`).

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