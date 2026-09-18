# Changelog

Mọi thay đổi đáng chú ý của dự án sẽ được ghi lại trong file này.

Định dạng theo [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
dự án tuân thủ [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Khung dự án ban đầu: cấu trúc `src/`, `Cargo.toml`, `docs/INSTALL.md`
- Tài liệu: `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`
- Giấy phép GPL-3.0
- CI workflow (GitHub Actions) cho Windows, Linux, macOS
- Explorer TUI cơ bản tại `src/main.rs`
- Script release nhanh `scripts/release.ps1`: commit, push, merge `main`, tạo tag, quay lại `develop` trong một lệnh