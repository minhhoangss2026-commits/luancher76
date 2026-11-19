use crate::core::mods;
use serde_json::Value;
use tauri::AppHandle;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

#[tauri::command]
pub async fn search_mods(query: String, version: String, loader: String) -> Result<Vec<Value>, String> {
    mods::search(&query, &version, &loader).await.map_err(err)
}

#[tauri::command]
pub async fn install_mod(app: AppHandle, project: String, version: String, loader: String) -> Result<Vec<String>, String> {
    mods::install(&app, &project, &version, &loader).await.map_err(err)
}

#[tauri::command]
pub async fn install_boost(app: AppHandle, version: String, loader: String) -> Result<Value, String> {
    mods::install_boost(&app, &version, &loader).await.map_err(err)
}

#[tauri::command]
pub fn list_mods(app: AppHandle, version: String, loader: String) -> Vec<Value> {
    mods::list_mods(&app, &loader, &version)
}

#[tauri::command]
pub fn toggle_mod(app: AppHandle, version: String, loader: String, file: String, enable: bool) -> bool {
    mods::set_enabled(&mods::mods_dir(&app, &loader, &version), &file, enable)
}

#[tauri::command]
pub fn apply_graphics(app: AppHandle, version: String, loader: String, preset: String) -> Result<(), String> {
    mods::apply_graphics(&app, &loader, &version, &preset).map_err(err)
}

#[tauri::command]
pub async fn search_curseforge(key: String, query: String, version: String, loader: String) -> Result<Vec<Value>, String> {
    crate::core::curse::search(&key, &query, &version, &loader).await.map_err(err)
}

#[tauri::command]
pub async fn install_curseforge(app: AppHandle, key: String, mod_id: String, version: String, loader: String) -> Result<Vec<String>, String> {
    crate::core::curse::install(&app, &key, &mod_id, &version, &loader).await.map_err(err)
}
