//! Forge / NeoForge: tải trình cài đặt chính thức rồi chạy `--installClient` vào thư mục game riêng.

use crate::core::{game, net, paths};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;
use tauri::AppHandle;

/// "1.20.4" -> (20, 4)
fn mc_parts(mc: &str) -> (u32, u32) {
    let mut it = mc.split('.');
    it.next();
    let minor = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let patch = it
        .next()
        .and_then(|x| x.split('-').next().and_then(|y| y.parse().ok()))
        .unwrap_or(0);
    (minor, patch)
}

fn key(v: &str) -> Vec<u64> {
    v.split(|c| c == '.' || c == '-').filter_map(|p| p.parse().ok()).collect()
}

/// Trả về (link trình cài, id phiên bản sẽ được tạo trong versions/)
async fn resolve(client: &reqwest::Client, loader: &str, mc: &str) -> Result<(String, String)> {
    let (minor, patch) = mc_parts(mc);
    match loader {
        "Forge" => {
            if minor < 13 {
                bail!("Forge chỉ hỗ trợ Minecraft 1.13 trở lên ở bản này");
            }
            let promos: Value = client
                .get("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json")
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let p = &promos["promos"];
            let ver = p[format!("{mc}-recommended")]
                .as_str()
                .or_else(|| p[format!("{mc}-latest")].as_str())
                .map(|s| s.to_string())
                .with_context(|| format!("Forge chưa có bản cho Minecraft {mc}"))?;
            Ok((
                format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{mc}-{ver}/forge-{mc}-{ver}-installer.jar"),
                format!("{mc}-forge-{ver}"),
            ))
        }
        "NeoForge" => {
            if (minor, patch) < (20, 2) {
                bail!("NeoForge chỉ hỗ trợ Minecraft 1.20.2 trở lên");
            }
            let prefix = format!("{minor}.{patch}.");
            let r: Value = client
                .get("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge")
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let best = r["versions"]
                .as_array()
                .context("danh sách NeoForge lỗi")?
                .iter()
                .filter_map(|v| v.as_str())
                .filter(|v| v.starts_with(&prefix) && !v.contains("beta") && !v.contains("alpha"))
                .max_by_key(|v| key(v))
                .map(|s| s.to_string())
                .with_context(|| format!("NeoForge chưa có bản cho Minecraft {mc}"))?;
            Ok((
                format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{best}/neoforge-{best}-installer.jar"),
                format!("neoforge-{best}"),
            ))
        }
        other => bail!("{other} không phải Forge/NeoForge"),
    }
}

/// Cài loader nếu chưa có, trả về id phiên bản (thư mục trong versions/).
pub async fn install(
    app: &AppHandle,
    client: &reqwest::Client,
    java: &Path,
    root: &Path,
    loader: &str,
    mc: &str,
) -> Result<String> {
    let (url, id) = resolve(client, loader, mc).await?;
    let json_path = root.join("versions").join(&id).join(format!("{id}.json"));
    if json_path.is_file() {
        return Ok(id);
    }
    tokio::fs::create_dir_all(root).await?;
    let profiles = root.join("launcher_profiles.json");
    if !profiles.exists() {
        tokio::fs::write(&profiles, br#"{"profiles":{},"version":3}"#).await?;
    }

    let installer = paths::runtime_dir(app).join("installers").join(format!("{id}-installer.jar"));
    game::emit(app, &format!("Tải trình cài {loader}"), 0, 1);
    net::download(client, &url, &installer, None, None, |_, _| {}).await?;

    game::emit(app, &format!("Cài {loader} (có thể mất vài phút)"), 0, 1);
    let mut cmd = tokio::process::Command::new(java);
    cmd.arg("-jar").arg(&installer).arg("--installClient").arg(root);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    let out = cmd.output().await.context("Không chạy được trình cài đặt")?;

    if !out.status.success() || !json_path.is_file() {
        let log = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut tail: Vec<&str> = log.lines().rev().filter(|l| !l.trim().is_empty()).take(12).collect();
        tail.reverse();
        // xóa file trình cài hỏng để lần sau tải lại
        let _ = std::fs::remove_file(&installer);
        bail!("Cài {loader} thất bại:\n{}", tail.join("\n"));
    }
    Ok(id)
}
