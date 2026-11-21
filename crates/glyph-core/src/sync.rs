use crate::models::{SyncBackend, SyncConfig, SyncMergeStats};
use crate::updater::ensure_rustls_ring_provider;
use crate::vault::VaultManager;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SYNC_FILENAME: &str = "glyph_sync_v3.glyph";

#[derive(Debug, Clone)]
pub struct SyncOutcome {
    pub stats: SyncMergeStats,
    pub synced_at: String,
    pub destination: String,
}

fn sync_local_folder(
    vault: &mut VaultManager,
    cfg: &SyncConfig,
    passphrase: &str,
) -> Result<SyncOutcome> {
    let folder = cfg.folder_path.trim();
    if folder.is_empty() {
        return Err(anyhow!("Local sync folder path is required"));
    }
    let dir = Path::new(folder);
    fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create sync folder {}", dir.display()))?;
    let target = dir.join(SYNC_FILENAME);

    let stats = if target.exists() {
        let existing = fs::read_to_string(&target)
            .with_context(|| format!("Failed to read {}", target.display()))?;
        if existing.trim().is_empty() {
            SyncMergeStats::default()
        } else {
            vault.merge_sync_snapshot(&existing, passphrase)?
        }
    } else {
        SyncMergeStats::default()
    };

    let updated_blob = vault.export_full_sync_snapshot(passphrase)?;
    let tmp = dir.join(format!(".{SYNC_FILENAME}.tmp"));
    fs::write(&tmp, updated_blob.as_bytes())?;
    if target.exists() {
        let _ = fs::remove_file(&target);
    }
    fs::rename(&tmp, &target)?;

    let now = Utc::now().to_rfc3339();
    vault.mark_synced_at(now.clone())?;
    Ok(SyncOutcome {
        stats,
        synced_at: now,
        destination: target.display().to_string(),
    })
}

fn webdav_client_and_url(cfg: &SyncConfig) -> Result<(reqwest::Client, String)> {
    ensure_rustls_ring_provider();
    let raw_url = cfg.webdav_url.trim().trim_end_matches('/');
    if raw_url.is_empty() {
        return Err(anyhow!("WebDAV endpoint URL is required"));
    }
    let file_url = if raw_url.ends_with(".glyph") {
        raw_url.to_string()
    } else {
        format!("{raw_url}/{SYNC_FILENAME}")
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Glyph-Desktop-v3")
        .build()?;
    Ok((client, file_url))
}

async fn webdav_get_remote(
    client: &reqwest::Client,
    file_url: &str,
    user: &str,
    pass: &str,
) -> Result<Option<String>> {
    let mut get_req = client.get(file_url);
    if !user.is_empty() {
        get_req = get_req.basic_auth(user, Some(pass));
    }
    let get_resp = get_req.send().await.context("WebDAV GET failed")?;
    if get_resp.status().is_success() {
        Ok(Some(get_resp.text().await?))
    } else if get_resp.status().as_u16() == 404 {
        Ok(None)
    } else {
        Err(anyhow!(
            "WebDAV GET returned unexpected status {}",
            get_resp.status()
        ))
    }
}

async fn webdav_put_remote(
    client: &reqwest::Client,
    file_url: &str,
    user: &str,
    pass: &str,
    body: String,
) -> Result<()> {
    let mut put_req = client
        .put(file_url)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(body);
    if !user.is_empty() {
        put_req = put_req.basic_auth(user, Some(pass));
    }
    let put_resp = put_req.send().await.context("WebDAV PUT failed")?;
    if !put_resp.status().is_success() {
        return Err(anyhow!("WebDAV PUT returned status {}", put_resp.status()));
    }
    Ok(())
}

pub async fn run_e2ee_sync(
    vault: &mut VaultManager,
    passphrase: &str,
    webdav_password_plain: Option<&str>,
) -> Result<SyncOutcome> {
    if passphrase.is_empty() {
        return Err(anyhow!("E2EE Sync passphrase is required"));
    }
    let cfg = vault.sync_config().clone();
    match cfg.backend {
        SyncBackend::None => Err(anyhow!("Sync backend is currently disabled")),
        SyncBackend::LocalFolder => sync_local_folder(vault, &cfg, passphrase),
        SyncBackend::WebDav => {
            let (client, file_url) = webdav_client_and_url(&cfg)?;
            let user = cfg.webdav_username.trim().to_string();
            let pass = match webdav_password_plain.filter(|s| !s.is_empty()) {
                Some(p) => p.to_string(),
                None => vault.decrypt_webdav_password()?.unwrap_or_default(),
            };
            let remote_blob = webdav_get_remote(&client, &file_url, &user, &pass).await?;
            let stats = match remote_blob.as_deref().map(str::trim) {
                Some(text) if !text.is_empty() => vault.merge_sync_snapshot(text, passphrase)?,
                _ => SyncMergeStats::default(),
            };
            let updated_blob = vault.export_full_sync_snapshot(passphrase)?;
            webdav_put_remote(&client, &file_url, &user, &pass, updated_blob).await?;
            let now = Utc::now().to_rfc3339();
            vault.mark_synced_at(now.clone())?;
            Ok(SyncOutcome {
                stats,
                synced_at: now,
                destination: file_url,
            })
        }
    }
}

/// Runs E2EE sync using short mutex critical sections so the vault lock is never
/// held across async WebDAV network operations.
pub async fn run_e2ee_sync_locked(
    vault: &Arc<Mutex<VaultManager>>,
    passphrase: &str,
    webdav_password_plain: Option<&str>,
) -> Result<SyncOutcome> {
    if passphrase.is_empty() {
        return Err(anyhow!("E2EE Sync passphrase is required"));
    }

    let (cfg, webdav_pass) = {
        let guard = vault.lock().map_err(|_| anyhow!("Vault mutex poisoned"))?;
        let cfg = guard.sync_config().clone();
        let pass = match webdav_password_plain.filter(|s| !s.is_empty()) {
            Some(p) => p.to_string(),
            None => guard.decrypt_webdav_password()?.unwrap_or_default(),
        };
        (cfg, pass)
    };

    match cfg.backend {
        SyncBackend::None => Err(anyhow!("Sync backend is currently disabled")),
        SyncBackend::LocalFolder => {
            let mut guard = vault.lock().map_err(|_| anyhow!("Vault mutex poisoned"))?;
            sync_local_folder(&mut guard, &cfg, passphrase)
        }
        SyncBackend::WebDav => {
            let (client, file_url) = webdav_client_and_url(&cfg)?;
            let user = cfg.webdav_username.trim().to_string();
            let remote_blob = webdav_get_remote(&client, &file_url, &user, &webdav_pass).await?;
            let (stats, updated_blob) = {
                let mut guard = vault.lock().map_err(|_| anyhow!("Vault mutex poisoned"))?;
                let stats = match remote_blob.as_deref().map(str::trim) {
                    Some(text) if !text.is_empty() => {
                        guard.merge_sync_snapshot(text, passphrase)?
                    }
                    _ => SyncMergeStats::default(),
                };
                let updated = guard.export_full_sync_snapshot(passphrase)?;
                (stats, updated)
            };
            webdav_put_remote(&client, &file_url, &user, &webdav_pass, updated_blob).await?;
            let now = Utc::now().to_rfc3339();
            let mut guard = vault.lock().map_err(|_| anyhow!("Vault mutex poisoned"))?;
            guard.mark_synced_at(now.clone())?;
            Ok(SyncOutcome {
                stats,
                synced_at: now,
                destination: file_url,
            })
        }
    }
}
