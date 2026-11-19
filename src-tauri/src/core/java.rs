use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};

/// Đảm bảo có JRE bản `major` (8/17/21...) cho đúng HĐH + CPU, tải từ Adoptium nếu thiếu.
pub async fn ensure_java<F>(
    client: &reqwest::Client,
    runtime_dir: &Path,
    major: u64,
    on_chunk: F,
) -> Result<PathBuf>
where
    F: FnMut(u64, Option<u64>),
{
    let dir = runtime_dir.join(format!("jre-{major}"));
    if let Some(j) = find_java(&dir, 0) {
        return Ok(j);
    }

    let (os, ext) = match std::env::consts::OS {
        "windows" => ("windows", "zip"),
        "macos" => ("mac", "tar.gz"),
        "linux" => ("linux", "tar.gz"),
        other => bail!("Hệ điều hành {other} chưa hỗ trợ tải Java tự động"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => bail!("CPU {other} chưa được hỗ trợ"),
    };

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major}/ga/{os}/{arch}/jre/hotspot/normal/eclipse"
    );
    tokio::fs::create_dir_all(runtime_dir).await?;
    let archive = runtime_dir.join(format!("jre-{major}.{ext}"));
    super::net::download(client, &url, &archive, None, None, on_chunk).await?;

    let a = archive.clone();
    let d = dir.clone();
    tokio::task::spawn_blocking(move || extract(&a, &d)).await??;
    let _ = std::fs::remove_file(&archive);

    find_java(&dir, 0).ok_or_else(|| anyhow!("Không tìm thấy java sau khi giải nén"))
}

fn extract(archive: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let file = std::fs::File::open(archive)?;
    if archive.to_string_lossy().ends_with(".zip") {
        let mut z = zip::ZipArchive::new(file)?;
        z.extract(dest).context("giải nén zip thất bại")?;
    } else {
        let gz = flate2::read::GzDecoder::new(file);
        let mut ar = tar::Archive::new(gz);
        ar.set_preserve_permissions(true);
        ar.unpack(dest).context("giải nén tar.gz thất bại")?;
    }
    Ok(())
}

/// Tìm file bin/java (hoặc bin/java.exe) trong thư mục đã giải nén.
fn find_java(dir: &Path, depth: u32) -> Option<PathBuf> {
    if depth > 6 || !dir.is_dir() {
        return None;
    }
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let direct = dir.join("bin").join(exe);
    if direct.is_file() {
        return Some(direct);
    }
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        if p.is_dir() {
            if let Some(found) = find_java(&p, depth + 1) {
                return Some(found);
            }
        }
    }
    None
}
