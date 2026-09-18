# AGENTS.md

Hướng dẫn cho agent khi làm việc trong dự án `editor`.

## Bối cảnh

- Đọc `README.md` để nắm mục tiêu dự án, `docs/INSTALL.md` cho cách build/cài đặt.
- Crate Rust đơn (`editor`), edition 2024, MSRV 1.85, dependency duy nhất `crossterm`.
- Mục tiêu TUI + GUI, giấy phép GPL-3.0 — phần đóng góp phải tương thích.

## Lệnh thường dùng

- Build: `cargo build`; bản release: `cargo build --release`
- Kiểm tra biên dịch: `cargo check`
- Chạy test: `cargo test` (hoặc `cargo test --package editor`)
- Format: `cargo fmt --check` (kiểm tra), `cargo fmt` (sửa)
- Lint: `cargo clippy --all-targets -- -D warnings`
- Chạy chương trình: `cargo run`

## Cấu trúc & gotcha

- Binary đơn tại `src/main.rs` (hiện là explorer TUI cơ bản); `src/terminal_tools.rs` và `src/terminal_ui_tools.rs` được khai báo `mod` trong `main.rs` nên nằm trong build.
- Nguồn CI là `.github/workflows/ci.yml` — chạy đúng 4 bước trong quy ước dưới cho Windows, Linux, macOS.

## Quy ước quan trọng

- **Không sửa `src/` trừ khi được yêu cầu trực tiếp** — mã nguồn đang ở giai đoạn giữ nguyên.
- **Thử nghiệm trên nhánh local, không push**: mỗi hướng thử nghiệm tạo nhánh local riêng (đặt tên có tiền tố như `dev/<tên>` hoặc `agents/<tên>`) từ `develop`; chỉ push nhánh lên GitHub khi được yêu cầu trực tiếp để remote luôn sạch. Nhánh local nằm trong `.git/refs/heads/`; kiểm tra nhánh remote bằng `git branch -r`.
- Mọi thay đổi phải pass đủ 4 bước CI: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`.
- Tuân thủ `CONTRIBUTING.md` khi đóng góp.
- Ghi thay đổi đáng chú ý vào `CHANGELOG.md` (đúng định dạng Keep a Changelog).
- Tài liệu viết tiếng Việt; mã nguồn, tên biến/hàm dùng tiếng Anh.

## Release nhanh

- Một lệnh cho toàn bộ pipeline: `.\scripts\release.ps1 "message commit"` — tự chạy 4 bước CI, commit sạch, push `develop`, merge `main`, tạo tag (tự tăng patch nếu không truyền `-Version`), push tag (kích hoạt release workflow), rồi quay lại `develop`.
- Tùy chọn: `-SkipChecks` (bỏ qua CI bước); `-Version v0.2.0` (gán tag cụ thể).