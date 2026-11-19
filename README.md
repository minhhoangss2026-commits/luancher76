# Cosmic Launcher (BETA)

Minecraft Launcher + FPS-Boost Client (Tauri v2 + React + Three.js).

## Đã có (Windows / macOS / Linux)
- Chạy Minecraft Vanilla, Fabric, Quilt, Forge (từ 1.13), NeoForge (từ 1.20.2); tự tải game + Java (Adoptium)
- Đăng nhập Microsoft (Device Code, cần Azure Client ID), tài khoản Offline, Khách
- FPS Boost: cờ JVM tối ưu, tự cài bộ mod FPS, preset đồ họa
- Kho Mod: Modrinth + CurseForge (cần API key), 1-click, tự cài dependency, bật/tắt mod
- AI Auto-Fixer (Anthropic / OpenAI / Gemini / Ollama) + chat
- Skin: xem 3D, áp vào game cho Offline/Khách bằng resource pack
- Quản lý Java, đổi thư mục dữ liệu, thông báo từ trang web (có lên lịch)

## Chưa có
Áo choàng trong game; chạy game trên Android / iOS (xem docs/MOBILE.md).

## Trang web thông báo
1. Sửa `WEB_URL` trong `src/config.ts` thành `https://<tên-github>.github.io/<tên-repo>/`.
2. Trên GitHub: Settings -> Pages -> Source = GitHub Actions.
3. Sửa `site/news.json` rồi push, workflow "Deploy web thông báo" sẽ đăng lên.

## Chạy thử giao diện
    npm install
    npm run dev

## Build
Tab Actions -> Build Windows / macOS / Android / iOS -> Run workflow -> tải ở Artifacts.
