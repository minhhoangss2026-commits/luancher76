use crate::core::{msauth, paths::data_dir};
use anyhow::{anyhow, Context};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Microsoft,
    Offline,
    Guest,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub uuid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
}

impl Account {
    /// Bản gửi cho giao diện: không chứa token
    fn public(&self) -> Account {
        let mut a = self.clone();
        a.refresh_token = None;
        a.access_token = None;
        a.expires_at = None;
        a.client_id = None;
        a
    }
}

/// UUID offline chuẩn Minecraft: UUIDv3 (MD5) của "OfflinePlayer:<name>"
fn offline_uuid(name: &str) -> String {
    let mut h = md5::compute(format!("OfflinePlayer:{name}").as_bytes()).0;
    h[6] = (h[6] & 0x0f) | 0x30;
    h[8] = (h[8] & 0x3f) | 0x80;
    Uuid::from_bytes(h).to_string()
}

fn file(app: &AppHandle) -> std::path::PathBuf {
    data_dir(app).join("accounts.json")
}

fn load(app: &AppHandle) -> Vec<Account> {
    std::fs::read_to_string(file(app))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(app: &AppHandle, list: &[Account]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(file(app), json).map_err(|e| e.to_string())
}

/// Lần mở đầu tiên: tự tạo hồ sơ Khách "Player_XXXX" để bấm PLAY ngay
#[tauri::command]
pub fn list_accounts(app: AppHandle) -> Result<Vec<Account>, String> {
    let mut list = load(&app);
    if list.is_empty() {
        let name = format!("Player_{:04}", rand::thread_rng().gen_range(0..10000));
        list.push(Account {
            id: Uuid::new_v4().to_string(),
            kind: Kind::Guest,
            uuid: offline_uuid(&name),
            name,
            skin_url: None,
            refresh_token: None,
            access_token: None,
            expires_at: None,
            client_id: None,
        });
        save(&app, &list)?;
    }
    Ok(list.iter().map(|a| a.public()).collect())
}

#[tauri::command]
pub fn add_offline_account(app: AppHandle, name: String) -> Result<Account, String> {
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 16 || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Tên chỉ gồm chữ, số, dấu _ (tối đa 16 ký tự)".into());
    }
    let mut list = load(&app);
    let acc = Account {
        id: Uuid::new_v4().to_string(),
        kind: Kind::Offline,
        uuid: offline_uuid(&name),
        name,
        skin_url: None,
        refresh_token: None,
        access_token: None,
        expires_at: None,
        client_id: None,
    };
    list.push(acc.clone());
    save(&app, &list)?;
    Ok(acc.public())
}

#[tauri::command]
pub fn remove_account(app: AppHandle, id: String) -> Result<(), String> {
    let mut list = load(&app);
    if list.len() <= 1 {
        return Err("Phải giữ lại ít nhất một tài khoản".into());
    }
    list.retain(|a| a.id != id);
    save(&app, &list)
}

#[tauri::command]
pub async fn ms_login_start(client_id: String) -> Result<Value, String> {
    if client_id.trim().is_empty() {
        return Err("Chưa nhập Azure Client ID ở tab Cài đặt".into());
    }
    msauth::device_start(client_id.trim()).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn ms_login_finish(
    app: AppHandle,
    client_id: String,
    device_code: String,
    interval: u64,
    expires_in: u64,
) -> Result<Account, String> {
    let s = msauth::device_finish(client_id.trim(), &device_code, interval, expires_in)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let mut list = load(&app);
    list.retain(|a| a.uuid != s.uuid);
    let acc = Account {
        id: Uuid::new_v4().to_string(),
        kind: Kind::Microsoft,
        name: s.name,
        uuid: s.uuid,
        skin_url: s.skin_url,
        refresh_token: Some(s.refresh_token),
        access_token: Some(s.access_token),
        expires_at: Some(s.expires_at),
        client_id: Some(client_id.trim().to_string()),
    };
    list.push(acc.clone());
    save(&app, &list)?;
    Ok(acc.public())
}

/// Lấy tài khoản đầy đủ (có token); tự làm mới token Microsoft nếu sắp hết hạn.
pub async fn fresh_account(app: &AppHandle, id: &str) -> anyhow::Result<Account> {
    let mut list = load(app);
    let idx = list
        .iter()
        .position(|a| a.id == id)
        .ok_or_else(|| anyhow!("Không tìm thấy tài khoản"))?;
    let acc = list[idx].clone();
    if acc.kind == Kind::Microsoft && acc.expires_at.unwrap_or(0) < msauth::now() + 120 {
        let cid = acc.client_id.clone().context("thiếu Client ID, hãy đăng nhập lại")?;
        let rt = acc.refresh_token.clone().context("thiếu refresh token, hãy đăng nhập lại")?;
        let s = msauth::refresh(&cid, &rt)
            .await
            .map_err(|e| anyhow!("Làm mới đăng nhập Microsoft thất bại, hãy đăng nhập lại: {e:#}"))?;
        let a = &mut list[idx];
        a.access_token = Some(s.access_token);
        a.refresh_token = Some(s.refresh_token);
        a.expires_at = Some(s.expires_at);
        a.name = s.name;
        let out = a.clone();
        save(app, &list).map_err(|e| anyhow!(e))?;
        return Ok(out);
    }
    Ok(acc)
}
