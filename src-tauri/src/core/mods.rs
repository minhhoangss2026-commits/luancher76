//! Modrinth: tìm mod, cài 1-click (kèm dependency bắt buộc), bộ mod FPS, bật/tắt mod, preset đồ họa.

use crate::core::{game, net, paths};
use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Bộ mod tăng FPS theo loader (Sodium/Lithium chỉ có cho Fabric/Quilt)
pub fn fps_mods(loader: &str) -> [&'static str; 5] {
    match loader {
        "Forge" | "NeoForge" => ["embeddium", "ferrite-core", "modernfix", "immediatelyfast", "entityculling"],
        _ => ["sodium", "lithium", "ferrite-core", "immediatelyfast", "entityculling"],
    }
}

pub fn profile_name(loader: &str, mc: &str) -> String {
    if loader == "Vanilla" { mc.to_string() } else { format!("{}-{}", loader.to_lowercase(), mc) }
}

pub fn instance(app: &AppHandle, loader: &str, mc: &str) -> PathBuf {
    paths::instance_dir(app, &profile_name(loader, mc))
}

pub fn mods_dir(app: &AppHandle, loader: &str, mc: &str) -> PathBuf {
    instance(app, loader, mc).join("mods")
}

pub fn manifest_path(app: &AppHandle, loader: &str, mc: &str) -> PathBuf {
    mods_dir(app, loader, mc).join(".cosmic.json")
}

fn loader_filter(loader: &str) -> Result<Vec<&'static str>> {
    match loader {
        "Fabric" => Ok(vec!["fabric"]),
        "Quilt" => Ok(vec!["quilt", "fabric"]),
        "Forge" => Ok(vec!["forge"]),
        "NeoForge" => Ok(vec!["neoforge"]),
        _ => bail!("Hãy chọn loader Fabric, Quilt, Forge hoặc NeoForge ở tab Chơi để dùng mod."),
    }
}

pub async fn search(query: &str, mc: &str, loader: &str) -> Result<Vec<Value>> {
    let client = net::client()?;
    let cats: Vec<Value> = loader_filter(loader)?
        .iter()
        .map(|l| json!(format!("categories:{l}")))
        .collect();
    let facets = json!([[format!("versions:{mc}")], cats, ["project_type:mod"]]).to_string();
    let resp: Value = client
        .get("https://api.modrinth.com/v2/search")
        .query(&[("query", query), ("limit", "24"), ("facets", facets.as_str())])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp["hits"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|h| {
            json!({
                "projectId": h["project_id"], "slug": h["slug"], "title": h["title"],
                "description": h["description"], "iconUrl": h["icon_url"],
                "downloads": h["downloads"], "author": h["author"],
            })
        })
        .collect())
}

pub fn read_manifest(path: &Path) -> Map<String, Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// Cài mod (project id hoặc slug) + dependency bắt buộc. Trả về danh sách file đã cài.
pub async fn install(app: &AppHandle, project: &str, mc: &str, loader: &str) -> Result<Vec<String>> {
    let client = net::client()?;
    let loaders = loader_filter(loader)?;
    let dir = mods_dir(app, loader, mc);
    tokio::fs::create_dir_all(&dir).await?;
    let mpath = manifest_path(app, loader, mc);
    let mut manifest = read_manifest(&mpath);

    let mut queue = vec![project.to_string()];
    let mut seen = HashSet::new();
    let mut installed = Vec::new();

    while let Some(p) = queue.pop() {
        if !seen.insert(p.clone()) {
            continue;
        }
        let versions: Value = client
            .get(format!("https://api.modrinth.com/v2/project/{p}/version"))
            .query(&[
                ("loaders", json!(loaders).to_string()),
                ("game_versions", json!([mc]).to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let arr = versions.as_array().cloned().unwrap_or_default();
        let chosen = arr
            .iter()
            .find(|x| x["version_type"] == "release")
            .or_else(|| arr.first());
        let Some(v) = chosen else {
            if p == project {
                bail!("Không có bản phù hợp của {p} cho Minecraft {mc} ({loader})");
            }
            continue;
        };
        let files = v["files"].as_array().cloned().unwrap_or_default();
        let Some(f) = files.iter().find(|f| f["primary"] == true).or_else(|| files.first()) else {
            continue;
        };
        let filename = f["filename"].as_str().context("thiếu tên file")?.to_string();
        let url = f["url"].as_str().context("thiếu url")?;
        let sha1 = f["hashes"]["sha1"].as_str();
        net::download(&client, url, &dir.join(&filename), sha1, f["size"].as_u64(), |_, _| {}).await?;

        // xóa bản cũ của cùng mod để tránh lỗi trùng mod
        let pid = v["project_id"].as_str().unwrap_or(&p).to_string();
        if let Some(old) = manifest.get(&pid).and_then(|o| o.as_str()) {
            if old != filename {
                let _ = std::fs::remove_file(dir.join(old));
            }
        }
        manifest.insert(pid, json!(filename));
        installed.push(filename);

        for d in v["dependencies"].as_array().into_iter().flatten() {
            if d["dependency_type"] == "required" {
                if let Some(id) = d["project_id"].as_str() {
                    queue.push(id.to_string());
                }
            }
        }
    }
    std::fs::write(&mpath, serde_json::to_vec_pretty(&Value::Object(manifest))?)?;
    Ok(installed)
}

/// Cài bộ mod tăng FPS. Mod nào chưa hỗ trợ phiên bản này sẽ được bỏ qua.
pub async fn install_boost(app: &AppHandle, mc: &str, loader: &str) -> Result<Value> {
    let mut ok = Vec::new();
    let mut skipped = Vec::new();
    let list = fps_mods(loader);
    for (i, slug) in list.iter().enumerate() {
        game::emit(app, "Cài mod FPS", i as u64, list.len() as u64);
        match install(app, slug, mc, loader).await {
            Ok(_) => ok.push(*slug),
            Err(_) => skipped.push(*slug),
        }
    }
    game::emit(app, "Cài mod FPS", list.len() as u64, list.len() as u64);
    Ok(json!({ "installed": ok, "skipped": skipped }))
}

pub fn list_mods(app: &AppHandle, loader: &str, mc: &str) -> Vec<Value> {
    let dir = mods_dir(app, loader, mc);
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".jar") {
                out.push(json!({ "file": name, "enabled": true }));
            } else if name.ends_with(".jar.disabled") {
                out.push(json!({ "file": name, "enabled": false }));
            }
        }
    }
    out.sort_by(|a, b| a["file"].as_str().cmp(&b["file"].as_str()));
    out
}

fn safe_name(f: &str) -> bool {
    !f.is_empty() && !f.contains('/') && !f.contains('\\') && !f.contains("..")
}

/// Bật/tắt mod bằng cách đổi đuôi .jar <-> .jar.disabled
pub fn set_enabled(dir: &Path, file: &str, enable: bool) -> bool {
    if !safe_name(file) {
        return false;
    }
    let (from, to) = if enable {
        (file.to_string(), file.trim_end_matches(".disabled").to_string())
    } else if file.ends_with(".disabled") {
        return true;
    } else {
        (file.to_string(), format!("{file}.disabled"))
    };
    std::fs::rename(dir.join(&from), dir.join(&to)).is_ok()
}

/// Ghi/trộn các khóa vào options.txt (preset đồ họa)
pub fn apply_graphics(app: &AppHandle, loader: &str, mc: &str, preset: &str) -> Result<()> {
    let dir = instance(app, loader, mc);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("options.txt");
    let vals: &[(&str, &str)] = if preset == "performance" {
        &[("renderDistance", "8"), ("simulationDistance", "6"), ("maxFps", "260"), ("enableVsync", "false"),
          ("particles", "2"), ("entityShadows", "false"), ("fancyGraphics", "false"), ("graphicsMode", "0"),
          ("ao", "false"), ("biomeBlendRadius", "1"), ("entityDistanceScaling", "0.75")]
    } else {
        &[("renderDistance", "12"), ("simulationDistance", "12"), ("maxFps", "120"), ("enableVsync", "true"),
          ("particles", "0"), ("entityShadows", "true"), ("fancyGraphics", "true"), ("graphicsMode", "1"),
          ("ao", "true"), ("biomeBlendRadius", "2"), ("entityDistanceScaling", "1.0")]
    };
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();
    for (k, v) in vals {
        let prefix = format!("{k}:");
        match lines.iter_mut().find(|l| l.starts_with(&prefix)) {
            Some(l) => *l = format!("{k}:{v}"),
            None => lines.push(format!("{k}:{v}")),
        }
    }
    std::fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}
