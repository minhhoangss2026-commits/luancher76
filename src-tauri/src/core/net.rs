use anyhow::{bail, Result};
use futures_util::StreamExt;
use sha1::{Digest, Sha1};
use std::path::Path;
use tokio::io::AsyncWriteExt;

pub fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("CosmicLauncher/0.1")
        .build()?)
}

/// Tải file về `dest`.
/// - Bỏ qua nếu file đã có và đúng dung lượng `size`.
/// - Kiểm tra SHA1 nếu có.
/// - Thử lại tối đa 3 lần.
pub async fn download<F>(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    sha1: Option<&str>,
    size: Option<u64>,
    mut on_chunk: F,
) -> Result<()>
where
    F: FnMut(u64, Option<u64>),
{
    if let Ok(meta) = std::fs::metadata(dest) {
        match size {
            Some(sz) if meta.len() == sz => return Ok(()),
            None if meta.len() > 0 => return Ok(()),
            _ => {}
        }
    }
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut last_err = None;
    for _ in 0..3 {
        match try_once(client, url, dest, sha1, &mut on_chunk).await {
            Ok(()) => return Ok(()),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap())
}

async fn try_once<F>(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    sha1: Option<&str>,
    on_chunk: &mut F,
) -> Result<()>
where
    F: FnMut(u64, Option<u64>),
{
    let resp = client.get(url).send().await?.error_for_status()?;
    let total = resp.content_length();

    let mut tmp_name = dest.as_os_str().to_owned();
    tmp_name.push(".part");
    let tmp = std::path::PathBuf::from(tmp_name);

    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut hasher = Sha1::new();
    let mut done: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        done += chunk.len() as u64;
        on_chunk(done, total);
    }
    file.flush().await?;
    drop(file);

    if let Some(expected) = sha1 {
        let got = format!("{:x}", hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&tmp).await;
            bail!("Sai SHA1 khi tải {url}");
        }
    }
    tokio::fs::rename(&tmp, dest).await?;
    Ok(())
}
