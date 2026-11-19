//! Engine tải + chạy Minecraft Vanilla trên Windows / macOS / Linux.
//! Luồng: Piston Meta -> client.jar -> libraries -> natives -> assets -> Java (Adoptium) -> chạy game.

use crate::commands::accounts::{Account, Kind};
use crate::core::{forge, java, net, paths};
use anyhow::{anyhow, bail, Context, Result};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

struct Download {
    url: String,
    path: PathBuf,
    sha1: Option<String>,
    size: Option<u64>,
}

// ------------------------------------------------------------------ tiện ích

pub fn emit(app: &AppHandle, stage: &str, cur: u64, total: u64) {
    let pct = if total > 0 { (cur as f64 / total as f64 * 100.0) as u32 } else { 0 };
    let _ = app.emit(
        "launch-progress",
        json!({ "stage": stage, "percent": pct, "current": cur, "total": total }),
    );
}

/// Callback tiến trình tải 1 file, chỉ bắn sự kiện khi % thay đổi (tránh spam).
fn throttled(app: AppHandle, stage: &'static str) -> impl FnMut(u64, Option<u64>) {
    let mut last = u32::MAX;
    move |done, total| {
        if let Some(t) = total {
            let p = (done * 100 / t.max(1)) as u32;
            if p != last {
                last = p;
                emit(&app, stage, done, t);
            }
        }
    }
}

async fn fetch_json(client: &reqwest::Client, url: &str) -> Result<Value> {
    Ok(client.get(url).send().await?.error_for_status()?.json::<Value>().await?)
}

fn os_name() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "osx",
        _ => "linux",
    }
}

/// 1 rule khớp máy hiện tại không? (feature rules như demo/custom resolution => không khớp)
fn rule_matches(rule: &Value) -> bool {
    if let Some(os) = rule.get("os") {
        if let Some(n) = os.get("name").and_then(|v| v.as_str()) {
            if n != os_name() {
                return false;
            }
        }
        if let Some(a) = os.get("arch").and_then(|v| v.as_str()) {
            let ok = match a {
                "x86" => std::env::consts::ARCH == "x86",
                "arm64" => std::env::consts::ARCH == "aarch64",
                _ => true,
            };
            if !ok {
                return false;
            }
        }
    }
    if rule.get("features").is_some() {
        return false;
    }
    true
}

fn allowed(rules: Option<&Value>) -> bool {
    let Some(arr) = rules.and_then(|r| r.as_array()) else { return true };
    if arr.is_empty() {
        return true;
    }
    let mut result = false;
    for r in arr {
        if rule_matches(r) {
            result = r.get("action").and_then(|a| a.as_str()) == Some("allow");
        }
    }
    result
}

fn subst(s: &str, m: &HashMap<&str, String>) -> String {
    let mut out = s.to_string();
    for (k, v) in m {
        out = out.replace(&format!("${{{k}}}"), v);
    }
    out
}

fn collect_args(list: Option<&Value>, m: &HashMap<&str, String>) -> Vec<String> {
    let mut out = Vec::new();
    let Some(arr) = list.and_then(|l| l.as_array()) else { return out };
    for item in arr {
        match item {
            Value::String(s) => out.push(subst(s, m)),
            Value::Object(_) => {
                if !allowed(item.get("rules")) {
                    continue;
                }
                match item.get("value") {
                    Some(Value::String(s)) => out.push(subst(s, m)),
                    Some(Value::Array(vs)) => {
                        for v in vs {
                            if let Some(s) = v.as_str() {
                                out.push(subst(s, m));
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

fn artifact_download(art: &Value, libs_dir: &Path) -> Option<Download> {
    Some(Download {
        url: art.get("url")?.as_str()?.to_string(),
        path: libs_dir.join(art.get("path")?.as_str()?),
        sha1: art.get("sha1").and_then(|v| v.as_str()).map(|s| s.to_string()),
        size: art.get("size").and_then(|v| v.as_u64()),
    })
}

/// Thư viện kiểu Maven không có mục "downloads" (Fabric/Quilt): tự suy ra đường dẫn từ tên.
fn maven_download(lib: &Value, libs_dir: &Path) -> Option<Download> {
    let name = lib.get("name")?.as_str()?;
    let p: Vec<&str> = name.split(':').collect();
    if p.len() < 3 {
        return None;
    }
    let (g, a, v) = (p[0].replace('.', "/"), p[1], p[2]);
    let cls = if p.len() >= 4 { format!("-{}", p[3]) } else { String::new() };
    let rel = format!("{g}/{a}/{v}/{a}-{v}{cls}.jar");
    let mut base = lib
        .get("url")
        .and_then(|u| u.as_str())
        .unwrap_or("https://libraries.minecraft.net/")
        .to_string();
    if !base.ends_with('/') {
        base.push('/');
    }
    Some(Download {
        url: format!("{base}{rel}"),
        path: libs_dir.join(&rel),
        sha1: None,
        size: None,
    })
}

fn lib_key(lib: &Value) -> String {
    let n = lib["name"].as_str().unwrap_or("");
    let p: Vec<&str> = n.split(':').collect();
    if p.len() >= 4 {
        format!("{}:{}:{}", p[0], p[1], p[3])
    } else if p.len() >= 2 {
        format!("{}:{}", p[0], p[1])
    } else {
        n.to_string()
    }
}

/// Gộp profile của Fabric/Quilt (child) vào JSON vanilla (parent).
fn merge(mut parent: Value, child: Value) -> Value {
    if let Some(mc) = child.get("mainClass").cloned() {
        parent["mainClass"] = mc;
    }
    let child_libs = child["libraries"].as_array().cloned().unwrap_or_default();
    let keys: HashSet<String> = child_libs.iter().map(lib_key).collect();
    let mut libs = child_libs;
    if let Some(pl) = parent["libraries"].as_array() {
        for l in pl {
            if !keys.contains(&lib_key(l)) {
                libs.push(l.clone());
            }
        }
    }
    parent["libraries"] = Value::Array(libs);
    for kind in ["game", "jvm"] {
        if let Some(ca) = child.get("arguments").and_then(|a| a.get(kind)).and_then(|v| v.as_array()) {
            if parent.get("arguments").is_none() {
                parent["arguments"] = json!({});
            }
            let entry = parent["arguments"]
                .as_object_mut()
                .unwrap()
                .entry(kind)
                .or_insert(json!([]));
            if let Some(arr) = entry.as_array_mut() {
                arr.extend(ca.iter().cloned());
            }
        }
    }
    parent
}

/// Lấy profile Fabric/Quilt mới nhất ổn định cho bản Minecraft `mc`.
async fn loader_profile(client: &reqwest::Client, loader: &str, mc: &str) -> Result<Value> {
    let base = match loader {
        "Fabric" => "https://meta.fabricmc.net/v2/versions/loader",
        "Quilt" => "https://meta.quiltmc.org/v3/versions/loader",
        other => bail!("{other} chưa được hỗ trợ"),
    };
    let list = fetch_json(client, &format!("{base}/{mc}")).await?;
    let arr = list.as_array().context("danh sách loader lỗi")?;
    if arr.is_empty() {
        bail!("{loader} chưa hỗ trợ Minecraft {mc}");
    }
    let pick = arr
        .iter()
        .find(|e| {
            let v = e["loader"]["version"].as_str().unwrap_or("");
            e["loader"]["stable"].as_bool() != Some(false)
                && !v.contains("beta")
                && !v.contains("pre")
        })
        .unwrap_or(&arr[0]);
    let lv = pick["loader"]["version"].as_str().context("thiếu version loader")?;
    fetch_json(client, &format!("{base}/{mc}/{lv}/profile/json")).await
}

/// Trả về (danh sách cần tải, classpath, jar natives kiểu cũ cần giải nén)
fn collect_libraries(vjson: &Value, libs_dir: &Path) -> (Vec<Download>, Vec<PathBuf>, Vec<PathBuf>) {
    let mut dls = Vec::new();
    let mut classpath: Vec<PathBuf> = Vec::new();
    let mut natives = Vec::new();
    let mut seen = HashSet::new();

    let Some(libs) = vjson.get("libraries").and_then(|l| l.as_array()) else {
        return (dls, classpath, natives);
    };
    for lib in libs {
        if !allowed(lib.get("rules")) {
            continue;
        }
        let downloads = lib.get("downloads");

        let main_dl = match downloads.and_then(|d| d.get("artifact")) {
            Some(art) => artifact_download(art, libs_dir),
            None if downloads.is_none() => maven_download(lib, libs_dir),
            None => None,
        };
        if let Some(d) = main_dl {
            if seen.insert(d.path.clone()) {
                classpath.push(d.path.clone());
                // thư viện do trình cài Forge/NeoForge tạo ra thì url rỗng: không tải
                if !d.url.is_empty() {
                    dls.push(d);
                }
            }
        }

        // natives kiểu cũ (<= 1.18): classifier riêng, phải giải nén
        if let Some(key) = lib
            .get("natives")
            .and_then(|n| n.get(os_name()))
            .and_then(|v| v.as_str())
        {
            let bits = if cfg!(target_pointer_width = "64") { "64" } else { "32" };
            let key = key.replace("${arch}", bits);
            if let Some(c) = downloads.and_then(|d| d.get("classifiers")).and_then(|c| c.get(&key)) {
                if let Some(d) = artifact_download(c, libs_dir) {
                    natives.push(d.path.clone());
                    dls.push(d);
                }
            }
        }
    }
    (dls, classpath, natives)
}

fn extract_natives(jars: &[PathBuf], dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for jar in jars {
        let f = std::fs::File::open(jar)?;
        let mut z = zip::ZipArchive::new(f)?;
        for i in 0..z.len() {
            let mut e = z.by_index(i)?;
            if e.is_dir() {
                continue;
            }
            let Some(name) = e.enclosed_name() else { continue };
            if name.starts_with("META-INF") {
                continue;
            }
            let out = dest.join(&name);
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p)?;
            }
            // file có thể đang bị khóa nếu game cũ còn chạy: bỏ qua lỗi
            if let Ok(mut o) = std::fs::File::create(&out) {
                let _ = std::io::copy(&mut e, &mut o);
            }
        }
    }
    Ok(())
}

async fn download_many(
    app: &AppHandle,
    client: &reqwest::Client,
    items: Vec<Download>,
    stage: &'static str,
    concurrency: usize,
) -> Result<()> {
    let total = items.len();
    if total == 0 {
        return Ok(());
    }
    let done = Arc::new(AtomicUsize::new(0));
    let results: Vec<Result<()>> = futures_util::stream::iter(items.into_iter().map(|d| {
        let client = client.clone();
        let app = app.clone();
        let done = done.clone();
        async move {
            net::download(&client, &d.url, &d.path, d.sha1.as_deref(), d.size, |_, _| {}).await?;
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 20 == 0 || n == total {
                emit(&app, stage, n as u64, total as u64);
            }
            Ok::<(), anyhow::Error>(())
        }
    }))
    .buffer_unordered(concurrency)
    .collect()
    .await;
    for r in results {
        r?;
    }
    Ok(())
}

// ------------------------------------------------------------------ API chính

/// Danh sách bản Release (mới nhất trước) để đổ vào ô chọn phiên bản.
pub async fn list_release_versions() -> Result<Vec<String>> {
    let client = net::client()?;
    let manifest = fetch_json(&client, MANIFEST_URL).await?;
    let list = manifest["versions"]
        .as_array()
        .context("manifest lỗi")?
        .iter()
        .filter(|v| v["type"].as_str() == Some("release"))
        .filter_map(|v| v["id"].as_str().map(|s| s.to_string()))
        .collect();
    Ok(list)
}

/// Tải đầy đủ mọi thứ còn thiếu rồi chạy game. Trả về ngay khi tiến trình game đã được khởi động.
pub async fn launch(
    app: &AppHandle,
    acc: &Account,
    version: &str,
    loader: &str,
    ram_mb: u32,
    boost: bool,
) -> Result<()> {
    if !matches!(std::env::consts::OS, "windows" | "macos" | "linux") {
        bail!("Android/iOS chưa hỗ trợ chạy game (cần runtime kiểu Pojav, sẽ làm ở module riêng)");
    }

    let client = net::client()?;
    let root = paths::game_root(app);
    let libs_dir = root.join("libraries");
    let assets_dir = root.join("assets");
    let versions_dir = root.join("versions").join(version);
    let natives_dir = versions_dir.join("natives");
    let instance = paths::instance_dir(app, &crate::core::mods::profile_name(loader, version));
    tokio::fs::create_dir_all(&instance).await?;
    tokio::fs::create_dir_all(&versions_dir).await?;

    // 1. Thông tin phiên bản (Piston Meta)
    emit(app, "Lấy thông tin phiên bản", 0, 1);
    let manifest = fetch_json(&client, MANIFEST_URL).await?;
    let vurl = manifest["versions"]
        .as_array()
        .and_then(|a| a.iter().find(|v| v["id"].as_str() == Some(version)))
        .and_then(|v| v["url"].as_str())
        .ok_or_else(|| anyhow!("Không tìm thấy phiên bản {version}"))?
        .to_string();
    let vjson = fetch_json(&client, &vurl).await?;
    tokio::fs::write(
        versions_dir.join(format!("{version}.json")),
        serde_json::to_vec_pretty(&vjson)?,
    )
    .await?;

    // 1b. Fabric / Quilt: gộp profile loader vào phiên bản vanilla
    let vjson = if matches!(loader, "Fabric" | "Quilt") {
        emit(app, "Tải bộ nạp mod", 0, 1);
        merge(vjson, loader_profile(&client, loader, version).await?)
    } else {
        vjson
    };

    // 2. client.jar
    let cd = &vjson["downloads"]["client"];
    let client_jar = versions_dir.join(format!("{version}.jar"));
    net::download(
        &client,
        cd["url"].as_str().context("thiếu url client")?,
        &client_jar,
        cd["sha1"].as_str(),
        cd["size"].as_u64(),
        throttled(app.clone(), "Tải game"),
    )
    .await?;

    // 2b. Java (cần sớm vì trình cài Forge/NeoForge chạy bằng Java)
    let mut major = vjson["javaVersion"]["majorVersion"].as_u64().unwrap_or(8);
    if major == 16 {
        major = 17; // Minecraft 1.17 chạy tốt trên Java 17
    }
    emit(app, "Chuẩn bị Java", 0, 1);
    let java_path = java::ensure_java(
        &client,
        &paths::runtime_dir(app),
        major,
        throttled(app.clone(), "Tải Java"),
    )
    .await?;

    // 2c. Forge / NeoForge: chạy trình cài rồi gộp profile vào vanilla
    let vjson = if matches!(loader, "Forge" | "NeoForge") {
        let id = forge::install(app, &client, &java_path, &root, loader, version).await?;
        let child_path = root.join("versions").join(&id).join(format!("{id}.json"));
        let child: Value = serde_json::from_slice(&tokio::fs::read(child_path).await?)?;
        merge(vjson, child)
    } else {
        vjson
    };

    // 3. Thư viện + natives
    let (lib_dls, lib_cp, native_jars) = collect_libraries(&vjson, &libs_dir);
    download_many(app, &client, lib_dls, "Tải thư viện", 8).await?;
    let nd = natives_dir.clone();
    tokio::task::spawn_blocking(move || extract_natives(&native_jars, &nd)).await??;

    // 4. Assets
    let ai = &vjson["assetIndex"];
    let asset_id = ai["id"].as_str().context("thiếu assetIndex.id")?.to_string();
    let idx_path = assets_dir.join("indexes").join(format!("{asset_id}.json"));
    net::download(
        &client,
        ai["url"].as_str().context("thiếu assetIndex.url")?,
        &idx_path,
        ai["sha1"].as_str(),
        ai["size"].as_u64(),
        |_, _| {},
    )
    .await?;
    let idx_json: Value = serde_json::from_slice(&tokio::fs::read(&idx_path).await?)?;
    let objects = idx_json["objects"].as_object().context("assetIndex lỗi")?;
    let mut seen = HashSet::new();
    let mut asset_items = Vec::new();
    for (_, o) in objects {
        let Some(hash) = o["hash"].as_str() else { continue };
        if !seen.insert(hash.to_string()) {
            continue;
        }
        let sub = &hash[..2];
        asset_items.push(Download {
            url: format!("https://resources.download.minecraft.net/{sub}/{hash}"),
            path: assets_dir.join("objects").join(sub).join(hash),
            sha1: Some(hash.to_string()),
            size: o["size"].as_u64(),
        });
    }
    download_many(app, &client, asset_items, "Tải assets", 24).await?;


    // 6. Ghép lệnh chạy
    emit(app, "Khởi động game", 0, 1);
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut cp: Vec<String> = lib_cp.iter().map(|p| p.to_string_lossy().to_string()).collect();
    cp.push(client_jar.to_string_lossy().to_string());
    let classpath = cp.join(sep);

    let mut m: HashMap<&str, String> = HashMap::new();
    m.insert("auth_player_name", acc.name.clone());
    m.insert("auth_uuid", acc.uuid.replace('-', ""));
    m.insert("auth_access_token", acc.access_token.clone().unwrap_or_else(|| "0".into()));
    m.insert("auth_xuid", "0".into());
    m.insert("clientid", "0".into());
    m.insert("user_type", if acc.kind == Kind::Microsoft { "msa".into() } else { "legacy".into() });
    m.insert("user_properties", "{}".into());
    m.insert("version_name", version.into());
    m.insert("version_type", vjson["type"].as_str().unwrap_or("release").into());
    m.insert("game_directory", instance.to_string_lossy().to_string());
    m.insert("assets_root", assets_dir.to_string_lossy().to_string());
    m.insert("game_assets", assets_dir.to_string_lossy().to_string());
    m.insert("assets_index_name", asset_id);
    m.insert("natives_directory", natives_dir.to_string_lossy().to_string());
    m.insert("library_directory", libs_dir.to_string_lossy().to_string());
    m.insert("classpath_separator", sep.into());
    m.insert("classpath", classpath.clone());
    m.insert("launcher_name", "CosmicLauncher".into());
    m.insert("launcher_version", "0.1.0".into());

    let mut jvm: Vec<String> = if boost {
        // Bộ cờ G1GC tối ưu giảm giật (áp dụng cho mọi nền tảng, gồm ARM64)
        let mut v: Vec<String> = vec![
            format!("-Xmx{ram_mb}M"),
            format!("-Xms{ram_mb}M"),
        ];
        for f in [
            "-XX:+UseG1GC", "-XX:+UnlockExperimentalVMOptions", "-XX:+DisableExplicitGC",
            "-XX:MaxGCPauseMillis=40", "-XX:G1NewSizePercent=30", "-XX:G1MaxNewSizePercent=40",
            "-XX:G1HeapRegionSize=8M", "-XX:G1ReservePercent=20", "-XX:G1HeapWastePercent=5",
            "-XX:G1MixedGCCountTarget=4", "-XX:InitiatingHeapOccupancyPercent=15",
            "-XX:G1MixedGCLiveThresholdPercent=90", "-XX:SurvivorRatio=32",
            "-XX:+PerfDisableSharedMem", "-XX:MaxTenuringThreshold=1",
        ] {
            v.push(f.to_string());
        }
        v
    } else {
        vec![
            format!("-Xmx{ram_mb}M"),
            format!("-Xms{}M", ram_mb.min(1024)),
            "-XX:+UseG1GC".into(),
            "-XX:MaxGCPauseMillis=50".into(),
        ]
    };
    let json_jvm = vjson.get("arguments").and_then(|a| a.get("jvm"));
    if json_jvm.is_some() {
        jvm.extend(collect_args(json_jvm, &m));
    } else {
        jvm.push(subst("-Djava.library.path=${natives_directory}", &m));
        jvm.push("-cp".into());
        jvm.push(classpath.clone());
    }
    // macOS + LWJGL3 bắt buộc chạy trên main thread
    if std::env::consts::OS == "macos"
        && classpath.contains("lwjgl-glfw")
        && !jvm.iter().any(|a| a == "-XstartOnFirstThread")
    {
        jvm.push("-XstartOnFirstThread".into());
    }

    let game_args: Vec<String> = if vjson.get("arguments").and_then(|a| a.get("game")).is_some() {
        collect_args(vjson["arguments"].get("game"), &m)
    } else {
        vjson["minecraftArguments"]
            .as_str()
            .unwrap_or("")
            .split_whitespace()
            .map(|s| subst(s, &m))
            .collect()
    };
    let main_class = vjson["mainClass"].as_str().context("thiếu mainClass")?;

    // 7. Chạy
    let mut cmd = tokio::process::Command::new(&java_path);
    cmd.args(&jvm)
        .arg(main_class)
        .args(&game_args)
        .current_dir(&instance)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // không bật cửa sổ console

    let mut child = cmd.spawn().context("Không khởi động được Java")?;

    if let Some(out) = child.stdout.take() {
        let a = app.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let _ = a.emit("game-log", l);
            }
        });
    }
    if let Some(err) = child.stderr.take() {
        let a = app.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(err).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let _ = a.emit("game-log", l);
            }
        });
    }
    let a = app.clone();
    tauri::async_runtime::spawn(async move {
        let code = child.wait().await.ok().and_then(|s| s.code());
        let _ = a.emit("game-exit", json!({ "code": code }));
    });

    emit(app, "Game đang chạy", 1, 1);
    Ok(())
}
