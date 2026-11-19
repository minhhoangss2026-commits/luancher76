//! Áp skin vào game cho tài khoản Offline/Khách bằng resource pack ghi đè skin mặc định.
//! (Tài khoản Microsoft dùng skin thật của Mojang; áo choàng không áp được bằng cách này.)

use crate::core::mods;
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Write;
use tauri::AppHandle;

const PACK: &str = "CosmicSkin.zip";
const ENTRY: &str = "file/CosmicSkin.zip";
const MCMETA: &str = r#"{"pack":{"pack_format":15,"description":"Cosmic Launcher skin","supported_formats":{"min_inclusive":1,"max_inclusive":999}}}"#;

/// `data_url = None` => gỡ resource pack skin.
pub fn apply(app: &AppHandle, loader: &str, mc: &str, data_url: Option<&str>) -> Result<()> {
    let inst = mods::instance(app, loader, mc);
    let rp_dir = inst.join("resourcepacks");
    let pack_path = rp_dir.join(PACK);

    let Some(data_url) = data_url else {
        let _ = std::fs::remove_file(pack_path);
        return Ok(());
    };
    let b64 = data_url.split(',').nth(1).context("Dữ liệu skin không hợp lệ")?;
    let png = STANDARD.decode(b64.trim())?;
    if png.len() > 1_000_000 || !png.starts_with(b"\x89PNG") {
        bail!("Skin phải là file PNG nhỏ hơn 1 MB");
    }

    std::fs::create_dir_all(&rp_dir)?;
    let mut z = zip::ZipWriter::new(std::fs::File::create(&pack_path)?);
    let opts = zip::write::SimpleFileOptions::default();
    z.start_file("pack.mcmeta", opts)?;
    z.write_all(MCMETA.as_bytes())?;
    // 1.19.3+: 9 skin mặc định x (wide|slim); bản cũ: steve/alex
    for n in ["steve", "alex", "ari", "efe", "kai", "makena", "noor", "sunny", "zuri"] {
        for kind in ["wide", "slim"] {
            z.start_file(format!("assets/minecraft/textures/entity/player/{kind}/{n}.png"), opts)?;
            z.write_all(&png)?;
        }
    }
    for n in ["steve", "alex"] {
        z.start_file(format!("assets/minecraft/textures/entity/{n}.png"), opts)?;
        z.write_all(&png)?;
    }
    z.finish()?;

    // bật pack trong options.txt
    let opt = inst.join("options.txt");
    let text = std::fs::read_to_string(&opt).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    match lines.iter_mut().find(|l| l.starts_with("resourcePacks:")) {
        Some(l) => {
            if !l.contains(ENTRY) {
                if let Some(pos) = l.rfind(']') {
                    let inner = l[..pos].to_string();
                    let sep = if inner.ends_with('[') { "" } else { "," };
                    *l = format!("{inner}{sep}\"{ENTRY}\"]");
                }
            }
        }
        None => lines.push(format!("resourcePacks:[\"vanilla\",\"{ENTRY}\"]")),
    }
    std::fs::write(opt, lines.join("\n") + "\n")?;
    Ok(())
}
