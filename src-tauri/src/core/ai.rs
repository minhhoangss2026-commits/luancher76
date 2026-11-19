//! Trợ lý AI: chat (Anthropic / OpenAI / Gemini / Ollama) + Auto-Fixer đọc log, vá mod, chỉnh RAM.

use crate::core::mods;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, SystemTime};
use tauri::AppHandle;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

#[derive(Deserialize, Clone)]
pub struct ChatMsg {
    pub role: String, // "user" | "assistant"
    pub content: String,
}

async fn post(client: &reqwest::Client, url: &str, headers: &[(&str, String)], body: Value) -> Result<Value> {
    let mut rb = client.post(url).json(&body);
    for (k, v) in headers {
        rb = rb.header(*k, v.as_str());
    }
    let resp = rb.send().await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        bail!("AI trả lỗi {status}: {}", text.chars().take(300).collect::<String>());
    }
    Ok(serde_json::from_str(&text)?)
}

pub async fn chat(cfg: &AiConfig, system: &str, msgs: &[ChatMsg]) -> Result<String> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(120)).build()?;
    let model = cfg.model.trim();
    let text = match cfg.provider.as_str() {
        "anthropic" => {
            let m = if model.is_empty() { "claude-sonnet-5-5" } else { model };
            let body = json!({
                "model": m, "max_tokens": 1500, "system": system,
                "messages": msgs.iter().map(|x| json!({"role": x.role, "content": x.content})).collect::<Vec<_>>(),
            });
            let r = post(&client, "https://api.anthropic.com/v1/messages",
                &[("x-api-key", cfg.api_key.clone()), ("anthropic-version", "2023-06-01".into())], body).await?;
            r["content"][0]["text"].as_str().unwrap_or("").to_string()
        }
        "openai" => {
            let m = if model.is_empty() { "gpt-4o-mini" } else { model };
            let base = if cfg.base_url.trim().is_empty() { "https://api.openai.com/v1" } else { cfg.base_url.trim() };
            let mut all = vec![json!({"role": "system", "content": system})];
            all.extend(msgs.iter().map(|x| json!({"role": x.role, "content": x.content})));
            let r = post(&client, &format!("{}/chat/completions", base.trim_end_matches('/')),
                &[("Authorization", format!("Bearer {}", cfg.api_key))], json!({"model": m, "messages": all})).await?;
            r["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string()
        }
        "gemini" => {
            let m = if model.is_empty() { "gemini-2.0-flash" } else { model };
            let contents: Vec<Value> = msgs.iter().map(|x| json!({
                "role": if x.role == "assistant" { "model" } else { "user" },
                "parts": [{"text": x.content}],
            })).collect();
            let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{m}:generateContent?key={}", cfg.api_key);
            let r = post(&client, &url, &[], json!({
                "systemInstruction": {"parts": [{"text": system}]}, "contents": contents,
            })).await?;
            r["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("").to_string()
        }
        "ollama" => {
            let m = if model.is_empty() { "llama3.1" } else { model };
            let base = if cfg.base_url.trim().is_empty() { "http://localhost:11434" } else { cfg.base_url.trim() };
            let mut all = vec![json!({"role": "system", "content": system})];
            all.extend(msgs.iter().map(|x| json!({"role": x.role, "content": x.content})));
            let r = post(&client, &format!("{}/api/chat", base.trim_end_matches('/')), &[],
                json!({"model": m, "messages": all, "stream": false})).await?;
            r["message"]["content"].as_str().unwrap_or("").to_string()
        }
        other => bail!("Nhà cung cấp AI không hợp lệ: {other}"),
    };
    if text.trim().is_empty() {
        bail!("AI không trả về nội dung");
    }
    Ok(text)
}

/// Đọc crash-report mới nhất (hoặc cuối latest.log) trong vòng 10 phút gần đây.
fn read_crash(instance: &Path) -> Option<String> {
    let fresh = |p: &Path| -> bool {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .map(|d| d < Duration::from_secs(600))
            .unwrap_or(false)
    };
    let mut newest: Option<(SystemTime, std::path::PathBuf)> = None;
    if let Ok(rd) = std::fs::read_dir(instance.join("crash-reports")) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "txt").unwrap_or(false) && fresh(&p) {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().map(|(n, _)| t > *n).unwrap_or(true) {
                        newest = Some((t, p));
                    }
                }
            }
        }
    }
    if let Some((_, p)) = newest {
        let s = std::fs::read_to_string(p).ok()?;
        return Some(s.chars().take(12000).collect());
    }
    let log = instance.join("logs").join("latest.log");
    if fresh(&log) {
        let s = std::fs::read_to_string(log).ok()?;
        let chars: Vec<char> = s.chars().collect();
        let start = chars.len().saturating_sub(12000);
        return Some(chars[start..].iter().collect());
    }
    None
}

const FIX_SYSTEM: &str = "You are an expert Minecraft crash fixer inside a launcher. \
Given a crash report/log, the loader, Minecraft version, RAM and the mod list, find the cause. \
Reply with ONLY one JSON object, no markdown: \
{\"diagnosis\":\"short explanation in Vietnamese\",\"actions\":[ \
{\"type\":\"disable_mod\",\"file\":\"exact jar filename from the mod list\"}, \
{\"type\":\"install_mod\",\"project\":\"Modrinth slug of a missing dependency or fix\"}, \
{\"type\":\"set_ram\",\"mb\":6144} ]}. \
Use at most 5 actions. Only use disable_mod with filenames that exist in the list. \
Use set_ram only for OutOfMemoryError. If you cannot fix it automatically, return an empty actions array.";

pub async fn autofix(app: &AppHandle, cfg: &AiConfig, mc: &str, loader: &str, ram_mb: u32) -> Result<Value> {
    let inst = mods::instance(app, loader, mc);
    let crash = read_crash(&inst).context("Không tìm thấy crash-report/log mới để phân tích")?;
    let list = mods::list_mods(app, loader, mc);
    let names: Vec<String> = list
        .iter()
        .map(|m| format!("{} ({})", m["file"].as_str().unwrap_or(""), if m["enabled"] == true { "on" } else { "off" }))
        .collect();
    let user = format!(
        "Minecraft: {mc}\nLoader: {loader}\nRAM: {ram_mb} MB\nMods:\n{}\n\nCrash/log:\n{crash}",
        if names.is_empty() { "(none)".to_string() } else { names.join("\n") }
    );
    let reply = chat(cfg, FIX_SYSTEM, &[ChatMsg { role: "user".into(), content: user }]).await?;

    let (a, b) = (reply.find('{'), reply.rfind('}'));
    let (Some(a), Some(b)) = (a, b) else { bail!("AI trả lời không đúng định dạng") };
    let parsed: Value = serde_json::from_str(&reply[a..=b]).context("Không đọc được JSON từ AI")?;

    let dir = mods::mods_dir(app, loader, mc);
    let mut applied: Vec<String> = Vec::new();
    let mut new_ram: Option<u32> = None;
    for act in parsed["actions"].as_array().cloned().unwrap_or_default().iter().take(5) {
        match act["type"].as_str() {
            Some("disable_mod") => {
                let f = act["file"].as_str().unwrap_or("");
                let exists = list.iter().any(|m| m["file"].as_str() == Some(f));
                if exists && mods::set_enabled(&dir, f, false) {
                    applied.push(format!("Tắt mod gây lỗi: {f}"));
                }
            }
            Some("install_mod") => {
                let p = act["project"].as_str().unwrap_or("");
                if !p.is_empty() && mods::install(app, p, mc, loader).await.is_ok() {
                    applied.push(format!("Tải/cài mod: {p}"));
                }
            }
            Some("set_ram") => {
                if let Some(mb) = act["mb"].as_u64() {
                    let mb = (mb as u32).clamp(1024, 32768);
                    new_ram = Some(mb);
                    applied.push(format!("Tăng RAM lên {mb} MB"));
                }
            }
            _ => {}
        }
    }
    Ok(json!({
        "diagnosis": parsed["diagnosis"].as_str().unwrap_or("(không có chẩn đoán)"),
        "applied": applied,
        "ramMb": new_ram,
    }))
}
