//! CurseForge API (cần API key của bạn từ console.curseforge.com).

use crate::core::{mods, net};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::HashSet;
use tauri::AppHandle;

fn cf_loader(loader: &str) -> Result<u32> {
    match loader {
        "Forge" => Ok(1),
        "Fabric" => Ok(4),
        "Quilt" => Ok(5),
        "NeoForge" => Ok(6),
        _ => bail!("Hãy chọn loader Fabric, Quilt, Forge hoặc NeoForge ở tab Chơi"),
    }
}

async fn get(client: &reqwest::Client, key: &str, url: &str, q: &[(&str, String)]) -> Result<Value> {
    if key.trim().is_empty() {
        bail!("Chưa nhập CurseForge API Key ở tab Cài đặt");
    }
    let resp = client
        .get(url)
        .header("x-api-key", key.trim())
        .header("Accept", "application/json")
        .query(q)
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        bail!("CurseForge trả lỗi {status}: {}", text.chars().take(200).collect::<String>());
    }
    Ok(serde_json::from_str(&text)?)
}

pub async fn search(key: &str, query: &str, mc: &str, loader: &str) -> Result<Vec<Value>> {
    let client = net::client()?;
    let lt = cf_loader(loader)?;
    let r = get(
        &client, key, "https://api.curseforge.com/v1/mods/search",
        &[
            ("gameId", "432".into()), ("classId", "6".into()), ("searchFilter", query.to_string()),
            ("gameVersion", mc.to_string()), ("modLoaderType", lt.to_string()),
            ("pageSize", "24".into()), ("sortField", "2".into()), ("sortOrder", "desc".into()),
        ],
    )
    .await?;
    Ok(r["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            json!({
                "projectId": m["id"].as_u64().unwrap_or(0).to_string(),
                "slug": m["slug"], "title": m["name"], "description": m["summary"],
                "iconUrl": m["logo"]["thumbnailUrl"], "downloads": m["downloadCount"],
                "author": m["authors"][0]["name"],
            })
        })
        .collect())
}

pub async fn install(app: &AppHandle, key: &str, mod_id: &str, mc: &str, loader: &str) -> Result<Vec<String>> {
    let client = net::client()?;
    let lt = cf_loader(loader)?;
    let dir = mods::mods_dir(app, loader, mc);
    tokio::fs::create_dir_all(&dir).await?;
    let mpath = mods::manifest_path(app, loader, mc);
    let mut manifest = mods::read_manifest(&mpath);

    let mut queue = vec![mod_id.to_string()];
    let mut seen = HashSet::new();
    let mut installed = Vec::new();

    while let Some(id) = queue.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let r = get(
            &client, key, &format!("https://api.curseforge.com/v1/mods/{id}/files"),
            &[("gameVersion", mc.to_string()), ("modLoaderType", lt.to_string()), ("pageSize", "10".into())],
        )
        .await?;
        let arr = r["data"].as_array().cloned().unwrap_or_default();
        let chosen = arr.iter().find(|f| f["releaseType"] == 1).or_else(|| arr.first());
        let Some(f) = chosen else {
            if id == mod_id {
                bail!("Không có bản phù hợp cho Minecraft {mc} ({loader})");
            }
            continue;
        };
        let Some(url) = f["downloadUrl"].as_str() else {
            if id == mod_id {
                bail!("Tác giả không cho phép tải ngoài CurseForge. Hãy tải thủ công trên trang CurseForge.");
            }
            continue;
        };
        let filename = f["fileName"].as_str().context("thiếu tên file")?.to_string();
        let sha1 = f["hashes"]
            .as_array()
            .and_then(|h| h.iter().find(|x| x["algo"] == 1))
            .and_then(|x| x["value"].as_str());
        net::download(&client, url, &dir.join(&filename), sha1, f["fileLength"].as_u64(), |_, _| {}).await?;

        let k = format!("cf-{id}");
        if let Some(old) = manifest.get(&k).and_then(|o| o.as_str()) {
            if old != filename {
                let _ = std::fs::remove_file(dir.join(old));
            }
        }
        manifest.insert(k, json!(filename));
        installed.push(filename);

        for d in f["dependencies"].as_array().into_iter().flatten() {
            if d["relationType"] == 3 {
                if let Some(m) = d["modId"].as_u64() {
                    queue.push(m.to_string());
                }
            }
        }
    }
    std::fs::write(&mpath, serde_json::to_vec_pretty(&serde_json::Value::Object(manifest))?)?;
    Ok(installed)
}
