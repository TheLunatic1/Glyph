use crate::models::{SyncBackend, SyncMergeStats};
use crate::updater::ensure_rustls_ring_provider;
use crate::vault::VaultManager;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::fs;
use std::path::Path;
use std::time::Duration;

const SYNC_FILENAME: &str = "glyph_sync_v3.glyph";

#[derive(Debug, Clone)]
pub struct SyncOutcome {
    pub stats: SyncMergeStats,
    pub synced_at: String,
    pub destination: String,
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
        SyncBackend::LocalFolder => {
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
        SyncBackend::WebDav => {
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

            let user = cfg.webdav_username.trim().to_string();
            let pass = match webdav_password_plain.filter(|s| !s.is_empty()) {
                Some(p) => p.to_string(),
                None => vault.decrypt_webdav_password()?.unwrap_or_default(),
            };

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent("Glyph-Desktop-v3")
                .build()?;

            let mut get_req = client.get(&file_url);
            if !user.is_empty() {
                get_req = get_req.basic_auth(&user, Some(&pass));
            }
            let get_resp = get_req.send().await.context("WebDAV GET failed")?;

            let stats = if get_resp.status().is_success() {
                let existing = get_resp.text().await?;
                if existing.trim().is_empty() {
                    SyncMergeStats::default()
                } else {
                    vault.merge_sync_snapshot(&existing, passphrase)?
                }
            } else if get_resp.status().as_u16() == 404 {
                SyncMergeStats::default()
            } else {
                return Err(anyhow!(
                    "WebDAV GET returned unexpected status {}",
                    get_resp.status()
                ));
            };

            let updated_blob = vault.export_full_sync_snapshot(passphrase)?;
            let mut put_req = client
                .put(&file_url)
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(updated_blob);
            if !user.is_empty() {
                put_req = put_req.basic_auth(&user, Some(&pass));
            }
            let put_resp = put_req.send().await.context("WebDAV PUT failed")?;
            if !put_resp.status().is_success() {
                return Err(anyhow!("WebDAV PUT returned status {}", put_resp.status()));
            }

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
