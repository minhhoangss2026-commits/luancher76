use crate::commands::accounts::fresh_account;
use crate::core::{game, mods, paths};
use tauri::AppHandle;

#[tauri::command]
pub async fn list_versions() -> Result<Vec<String>, String> {
    game::list_release_versions().await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn launch_game(
    app: AppHandle,
    account_id: String,
    version: String,
    loader: String,
    ram_mb: u32,
    boost: bool,
) -> Result<(), String> {
    if !matches!(loader.as_str(), "Vanilla" | "Fabric" | "Quilt" | "Forge" | "NeoForge") {
        return Err(format!("{loader} chưa được hỗ trợ ở bản này (chọn Vanilla, Fabric, Quilt, Forge hoặc NeoForge)"));
    }
    let acc = fresh_account(&app, &account_id).await.map_err(|e| format!("{e:#}"))?;

    // Lần đầu dùng loader mod + bật FPS Boost: tự cài bộ mod tăng FPS
    if boost && loader != "Vanilla" && !mods::manifest_path(&app, &loader, &version).exists() {
        mods::install_boost(&app, &version, &loader)
            .await
            .map_err(|e| format!("{e:#}"))?;
    }
    game::launch(&app, &acc, &version, &loader, ram_mb, boost)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn get_data_dir(app: AppHandle) -> String {
    paths::data_dir(&app).to_string_lossy().to_string()
}

#[tauri::command]
pub fn set_data_dir(app: AppHandle, path: String) -> Result<(), String> {
    paths::set_data_dir(&app, &path)
}

#[tauri::command]
pub fn list_runtimes(app: AppHandle) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(paths::runtime_dir(&app))
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|n| n.starts_with("jre-"))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[tauri::command]
pub fn remove_runtime(app: AppHandle, name: String) -> Result<(), String> {
    if !name.starts_with("jre-") || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Tên Java không hợp lệ".into());
    }
    std::fs::remove_dir_all(paths::runtime_dir(&app).join(name)).map_err(|e| e.to_string())
}

/// Áp skin cho tài khoản offline/khách (data URL PNG); None = gỡ
#[tauri::command]
pub fn apply_skin(app: AppHandle, version: String, loader: String, data_url: Option<String>) -> Result<(), String> {
    crate::core::skin::apply(&app, &loader, &version, data_url.as_deref()).map_err(|e| format!("{e:#}"))
}
