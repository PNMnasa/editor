# Đóng góp

Cảm ơn bạn đã quan tâm đến dự án. Trước khi đóng góp, vui lòng đọc qua các quy ước sau.

## Yêu cầu môi trường

- Rust toolchain từ **1.85** trở lên — cài qua [rustup](https://rustup.rs/)
- Đảm bảo `cargo`, `rustc` hoạt động từ terminal

## Quy trình đóng góp

1. **Mở issue** mô tả vấn đề hoặc tính năng bạn muốn thực hiện, để trao đổi trước khi code.
2. **Fork** repository và tạo nhánh riêng:

   ```sh
   git checkout -b feature/tinh-nang-moi
   ```

3. Thực hiện thay đổi, đảm bảo:
   - `cargo fmt` — format chuẩn
   - `cargo clippy -- -D warnings` — không còn cảnh báo
   - `cargo test` — các test đều pass
   - `cargo check` — biên dịch không lỗi
4. **Commit** với thông điệp rõ ràng, tóm tắt thay đổi.
5. **Pull request** lên nhánh chính, mô tả nội dung thay đổi và kết quả kiểm tra.

## Quy tắc viết code

- Giữ code gọn, dễ đọc; ưu tiên thư viện có sẵn trong dự án thay vì thêm dependency mới khi không cần.
- Thêm test cho logic mới khi có thể.
- Không đăng ký thông tin nhạy cảm, secret hay key.

## Giấy phép

Dự án phát hành theo [GPL-3.0](LICENSE). Khi đóng góp, bạn đồng ý phần đóng góp của mình được cấp phép theo GPL-3.0.