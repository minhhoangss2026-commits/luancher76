//! Đăng nhập Microsoft (Device Code Flow) -> Xbox Live -> XSTS -> Minecraft Services.
//! Cần Azure Client ID của bạn (ứng dụng public client) đã được Mojang cho phép dùng Minecraft API.

use crate::core::net;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SCOPE: &str = "XboxLive.signin offline_access";

pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
    pub uuid: String,
    pub name: String,
    pub skin_url: Option<String>,
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

async fn post_json(client: &reqwest::Client, url: &str, body: Value) -> Result<Value> {
    let resp = client.post(url).header("Accept", "application/json").json(&body).send().await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        if text.contains("2148916233") {
            bail!("Tài khoản Microsoft này chưa có hồ sơ Xbox. Hãy tạo tại xbox.com rồi thử lại.");
        }
        if text.contains("2148916238") {
            bail!("Tài khoản trẻ em cần được người lớn thêm vào gia đình Microsoft trước.");
        }
        bail!("Lỗi {status}: {}", text.chars().take(300).collect::<String>());
    }
    Ok(serde_json::from_str(&text)?)
}

/// Bước 1: xin mã thiết bị. Người dùng mở verificationUri và nhập userCode.
pub async fn device_start(client_id: &str) -> Result<Value> {
    let client = net::client()?;
    let r: Value = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .form(&[("client_id", client_id), ("scope", SCOPE)])
        .send()
        .await?
        .json()
        .await?;
    if r.get("device_code").is_none() {
        bail!("Microsoft từ chối: {}", r["error_description"].as_str().unwrap_or("hãy kiểm tra Client ID"));
    }
    Ok(json!({
        "userCode": r["user_code"], "verificationUri": r["verification_uri"],
        "deviceCode": r["device_code"], "interval": r["interval"], "expiresIn": r["expires_in"],
    }))
}

/// Bước 2: chờ người dùng xác nhận rồi đổi sang phiên Minecraft.
pub async fn device_finish(client_id: &str, device_code: &str, interval: u64, expires_in: u64) -> Result<Session> {
    let client = net::client()?;
    let deadline = now() + expires_in.max(60);
    let mut wait = interval.max(3);
    loop {
        if now() > deadline {
            bail!("Hết thời gian chờ đăng nhập, hãy thử lại");
        }
        tokio::time::sleep(Duration::from_secs(wait)).await;
        let r: Value = client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", device_code),
            ])
            .send()
            .await?
            .json()
            .await?;
        if let Some(at) = r["access_token"].as_str() {
            let rt = r["refresh_token"].as_str().unwrap_or("").to_string();
            return minecraft_session(at, rt).await;
        }
        match r["error"].as_str() {
            Some("authorization_pending") => {}
            Some("slow_down") => wait += 5,
            _ => bail!("Đăng nhập thất bại: {}", r["error_description"].as_str().unwrap_or("lỗi không rõ")),
        }
    }
}

/// Làm mới token bằng refresh_token (tự động trước khi chơi).
pub async fn refresh(client_id: &str, refresh_token: &str) -> Result<Session> {
    let client = net::client()?;
    let r: Value = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ])
        .send()
        .await?
        .json()
        .await?;
    let at = r["access_token"]
        .as_str()
        .with_context(|| format!("{}", r["error_description"].as_str().unwrap_or("refresh thất bại")))?;
    let rt = r["refresh_token"].as_str().unwrap_or(refresh_token).to_string();
    minecraft_session(at, rt).await
}

async fn minecraft_session(ms_access: &str, ms_refresh: String) -> Result<Session> {
    let client = net::client()?;
    let xbl = post_json(
        &client,
        "https://user.auth.xboxlive.com/user/authenticate",
        json!({
            "Properties": {"AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={ms_access}")},
            "RelyingParty": "http://auth.xboxlive.com", "TokenType": "JWT",
        }),
    )
    .await?;
    let xbl_token = xbl["Token"].as_str().context("thiếu XBL token")?.to_string();
    let uhs = xbl["DisplayClaims"]["xui"][0]["uhs"].as_str().context("thiếu uhs")?.to_string();

    let xsts = post_json(
        &client,
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        json!({
            "Properties": {"SandboxId": "RETAIL", "UserTokens": [xbl_token]},
            "RelyingParty": "rp://api.minecraftservices.com/", "TokenType": "JWT",
        }),
    )
    .await?;
    let xsts_token = xsts["Token"].as_str().context("thiếu XSTS token")?;

    let mc = post_json(
        &client,
        "https://api.minecraftservices.com/authentication/login_with_xbox",
        json!({"identityToken": format!("XBL3.0 x={uhs};{xsts_token}")}),
    )
    .await?;
    let token = mc["access_token"].as_str().context("thiếu access_token Minecraft")?.to_string();
    let expires = mc["expires_in"].as_u64().unwrap_or(86400);

    let prof = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&token)
        .send()
        .await?;
    if prof.status() == reqwest::StatusCode::NOT_FOUND {
        bail!("Tài khoản Microsoft này chưa mua Minecraft Java Edition");
    }
    let prof: Value = prof.error_for_status()?.json().await?;
    let id = prof["id"].as_str().context("thiếu id hồ sơ")?;
    let uuid = if id.len() == 32 {
        format!("{}-{}-{}-{}-{}", &id[0..8], &id[8..12], &id[12..16], &id[16..20], &id[20..32])
    } else {
        id.to_string()
    };
    let skin_url = prof["skins"]
        .as_array()
        .and_then(|a| a.iter().find(|s| s["state"] == "ACTIVE"))
        .and_then(|s| s["url"].as_str())
        .map(|s| s.to_string());

    Ok(Session {
        access_token: token,
        refresh_token: ms_refresh,
        expires_at: now() + expires,
        uuid,
        name: prof["name"].as_str().unwrap_or("Player").to_string(),
        skin_url,
    })
}
