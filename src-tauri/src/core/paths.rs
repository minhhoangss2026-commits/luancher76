use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn default_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("không xác định được app_data_dir")
        .join("CosmicLauncher")
}

/// Thư mục dữ liệu RIÊNG của launcher (không đụng ~/.minecraft).
/// Người dùng có thể đổi sang thư mục khác (lưu trong data_dir.txt ở thư mục mặc định).
pub fn data_dir(app: &AppHandle) -> PathBuf {
    let def = default_dir(app);
    let dir = std::fs::read_to_string(def.join("data_dir.txt"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or(def);
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Đặt thư mục dữ liệu mới (chuỗi rỗng = quay về mặc định). Có hiệu lực sau khi khởi động lại.
pub fn set_data_dir(app: &AppHandle, path: &str) -> Result<(), String> {
    let def = default_dir(app);
    std::fs::create_dir_all(&def).map_err(|e| e.to_string())?;
    let f = def.join("data_dir.txt");
    if path.trim().is_empty() {
        let _ = std::fs::remove_file(f);
        return Ok(());
    }
    std::fs::create_dir_all(path.trim()).map_err(|e| format!("Không tạo được thư mục: {e}"))?;
    std::fs::write(f, path.trim()).map_err(|e| e.to_string())
}

/// libraries / assets / versions dùng chung giữa các profile
pub fn game_root(app: &AppHandle) -> PathBuf {
    data_dir(app).join("minecraft")
}

/// Java Runtime tự tải về
pub fn runtime_dir(app: &AppHandle) -> PathBuf {
    data_dir(app).join("runtime")
}

/// Thư mục game riêng của từng phiên bản (saves, mods, config...)
pub fn instance_dir(app: &AppHandle, name: &str) -> PathBuf {
    data_dir(app).join("instances").join(name)
}
