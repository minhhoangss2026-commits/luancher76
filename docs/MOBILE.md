# Android / iOS

Bản này chạy được giao diện trên Android/iOS (Tauri v2 mobile), nhưng KHÔNG chạy được Minecraft Java trên điện thoại.

## Vì sao
Máy tính chạy game bằng cách gọi `java`. Điện thoại không có JVM sẵn và không có OpenGL desktop. Cần một runtime riêng gồm:
- JVM biên dịch cho ARM (Android: JNI + JRE ARM64; iOS: JIT, chỉ sideload được qua AltStore/SideStore/JitStreamer)
- Lớp dịch OpenGL -> GLES/Vulkan (kiểu gl4es / MobileGlues)
- Lớp GLFW giả lập (cửa sổ, cảm ứng, tay cầm)
- Điều khiển cảm ứng ảo

Đây chính là phần mà PojavLauncher / Amethyst / Zalith đã làm (mã nguồn mở, giấy phép LGPL-3.0).

## Hướng đi thực tế
1. Dùng một trong các dự án trên làm runtime, nhúng như plugin Tauri (Kotlin cho Android, Swift cho iOS).
2. Viết lại `GameRuntime::launch` cho `target_os = "android"` / `"ios"` để gọi runtime đó, dùng chung tài khoản, mod, Auto-Fixer.
3. Giữ nguyên toàn bộ giao diện hiện tại.

Phần này là một dự án native riêng, cần máy thật để thử nghiệm.
