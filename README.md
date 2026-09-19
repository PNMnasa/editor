# Editor for anything

## Về dự án

Editor cho mọi thứ — hỗ trợ code, chỉnh sửa ảnh và các định dạng phổ biến khác. Dự án nằm trong kế hoạch xây dựng nền tảng lớn hơn: dung lượng file cài đặt, RAM/CPU/GPU sử dụng có thể giảm đáng kể do dùng chung tài nguyên.

## Tính năng

### Đã hoàn thành

- Manager: Xem tên, dung lượng file folder; Việc tính toán dung lượng gây delay giao diện

### Chỉnh sửa
- Code: soạn thảo và hỗ trợ nhiều ngôn ngữ lập trình
- Ảnh: chỉnh sửa ảnh
- Các định dạng phổ biến khác

### Giao diện
- Hiển thị GUI và TUI

### Tích hợp
- Gợi ý CLI, chỉnh sửa bằng MCP, Skills

### Hướng phát triển
- Xây dựng cho con người, có hướng dành riêng cho Agents (server) trong tương lai

## Cài đặt

Hướng dẫn cài đặt và build chi tiết tại [docs/INSTALL.md](docs/INSTALL.md).

## Công nghệ

- **Rust** — tối ưu native, biên dịch chéo dễ dàng sang nhiều hệ điều hành, kể cả các hệ điều hành ngách nhất
- **GUI layer tự viết** — đồng bộ giữa GUI và TUI, cho phép tối ưu sâu cho từng nền tảng
- **Hướng tối ưu native** — dung lượng nhỏ, tiêu thụ RAM/CPU/GPU thấp