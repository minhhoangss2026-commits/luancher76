use crate::core::ai::{self, AiConfig, ChatMsg};
use serde_json::Value;
use tauri::AppHandle;

#[tauri::command]
pub async fn ai_chat(cfg: AiConfig, system: String, messages: Vec<ChatMsg>) -> Result<String, String> {
    ai::chat(&cfg, &system, &messages).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn ai_autofix(app: AppHandle, cfg: AiConfig, version: String, loader: String, ram_mb: u32) -> Result<Value, String> {
    ai::autofix(&app, &cfg, &version, &loader, ram_mb).await.map_err(|e| format!("{e:#}"))
}
