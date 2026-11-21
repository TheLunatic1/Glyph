use crate::ssh::{posix_quote, SshSession};
use anyhow::{anyhow, Context, Result};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::FileAttributes;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_EDIT_FILE_BYTES: usize = 5 * 1024 * 1024;
const TRANSFER_CHUNK_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpEntry {
    pub filename: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub permissions: u32,
    pub modified: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Upload,
    Download,
    RemoteCopy,
}

#[derive(Debug, Clone)]
pub struct SftpProgress {
    pub transfer_id: String,
    pub kind: TransferKind,
    pub filename: String,
    pub transferred: u64,
    pub total: u64,
}

pub fn join_remote_path(dir: &str, name: &str) -> String {
    let trimmed_name = name.trim_start_matches('/');
    if dir.is_empty() || dir == "/" {
        format!("/{trimmed_name}")
    } else {
        format!("{}/{trimmed_name}", dir.trim_end_matches('/'))
    }
}

pub fn format_permissions_octal(perms: u32) -> String {
    format!("{:03o}", perms & 0o777)
}

impl SshSession {
    pub async fn sftp_session(&self) -> Result<Arc<SftpSession>> {
        let mut slot = self.inner.sftp.lock().await;
        if let Some(existing) = slot.as_ref() {
            return Ok(Arc::clone(existing));
        }

        let channel = {
            let guard = self.inner.handle.lock().await;
            let handle = guard.as_ref().ok_or_else(|| anyhow!("Not connected"))?;
            handle
                .channel_open_session()
                .await
                .context("Failed to open SSH session channel for SFTP")?
        };
        channel
            .request_subsystem(true, "sftp")
            .await
            .context("Failed to start SFTP subsystem")?;

        let session = SftpSession::new(channel.into_stream())
            .await
            .context("SFTP handshake failed")?;
        session.set_timeout(30);
        let arc = Arc::new(session);
        *slot = Some(Arc::clone(&arc));
        Ok(arc)
    }

    pub async fn reset_sftp_session(&self) {
        if let Some(old) = self.inner.sftp.lock().await.take() {
            let _ = old.close().await;
        }
    }

    pub async fn sftp_readdir(&self, path: &str) -> Result<Vec<SftpEntry>> {
        let sftp = self.sftp_session().await?;
        let dir = match sftp.read_dir(path).await {
            Ok(d) => d,
            Err(_) => {
                // Retry once with a fresh SFTP channel in case the server closed idle subsystem
                self.reset_sftp_session().await;
                let fresh = self.sftp_session().await?;
                fresh
                    .read_dir(path)
                    .await
                    .with_context(|| format!("Failed to list directory {path}"))?
            }
        };

        let mut entries: Vec<SftpEntry> = dir
            .into_iter()
            .map(|entry| {
                let meta = entry.metadata();
                let filename = entry.file_name();
                SftpEntry {
                    path: join_remote_path(path, &filename),
                    filename,
                    is_dir: meta.is_dir(),
                    is_symlink: meta.is_symlink(),
                    size: meta.size.unwrap_or(0),
                    permissions: meta.permissions.unwrap_or(0) & 0o7777,
                    modified: meta.mtime,
                }
            })
            .collect();

        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.filename.to_ascii_lowercase().cmp(&b.filename.to_ascii_lowercase()))
        });
        Ok(entries)
    }

    pub async fn sftp_read_text(&self, path: &str) -> Result<String> {
        let sftp = self.sftp_session().await?;
        if let Ok(meta) = sftp.metadata(path).await {
            if meta.size.unwrap_or(0) as usize > MAX_EDIT_FILE_BYTES {
                return Err(anyhow!(
                    "File exceeds {} MB inline editor limit; use Download instead.",
                    MAX_EDIT_FILE_BYTES / (1024 * 1024)
                ));
            }
        }
        let mut file = sftp
            .open(path)
            .await
            .with_context(|| format!("Failed to open remote file {path}"))?;
        let mut buf = Vec::new();
        let mut chunk = [0u8; TRANSFER_CHUNK_BYTES];
        loop {
            let n = file.read(&mut chunk).await?;
            if n == 0 {
                break;
            }
            if buf.len() + n > MAX_EDIT_FILE_BYTES {
                let _ = file.close().await;
                return Err(anyhow!("File exceeds 5 MB inline editor limit"));
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        let _ = file.close().await;
        String::from_utf8(buf)
            .map_err(|_| anyhow!("File is binary or non-UTF8 and cannot be edited inline"))
    }

    pub async fn sftp_write_text(&self, path: &str, content: &str) -> Result<()> {
        let sftp = self.sftp_session().await?;
        let mut file = sftp
            .create(path)
            .await
            .with_context(|| format!("Failed to open {path} for writing"))?;
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;
        file.close().await?;
        Ok(())
    }

    pub async fn sftp_upload_file(
        &self,
        local_path: &Path,
        remote_path: &str,
        progress_tx: Option<mpsc::UnboundedSender<SftpProgress>>,
    ) -> Result<u64> {
        self.sftp_upload_file_with_cancel(
            local_path,
            remote_path,
            Uuid::new_v4().to_string(),
            progress_tx,
            None,
        )
        .await
    }

    pub async fn sftp_upload_file_with_cancel(
        &self,
        local_path: &Path,
        remote_path: &str,
        transfer_id: String,
        progress_tx: Option<mpsc::UnboundedSender<SftpProgress>>,
        cancel: Option<CancellationToken>,
    ) -> Result<u64> {
        let meta = tokio::fs::metadata(local_path)
            .await
            .with_context(|| format!("Cannot stat local file {}", local_path.display()))?;
        let total = meta.len();
        let filename = local_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("upload")
            .to_string();

        let sftp = self.sftp_session().await?;
        let mut local = tokio::fs::File::open(local_path).await?;
        let mut remote = sftp
            .create(remote_path)
            .await
            .with_context(|| format!("Failed to create remote file {remote_path}"))?;

        let copied = stream_copy_with_progress(
            &mut local,
            &mut remote,
            TransferKind::Upload,
            &transfer_id,
            &filename,
            total,
            progress_tx.as_ref(),
            cancel.as_ref(),
        )
        .await;

        match copied {
            Ok(transferred) => {
                remote.flush().await?;
                remote.close().await?;
                emit_done(progress_tx.as_ref(), transfer_id, TransferKind::Upload, filename, transferred, total);
                Ok(transferred)
            }
            Err(err) => {
                let _ = remote.close().await;
                let _ = sftp.remove_file(remote_path).await;
                Err(err)
            }
        }
    }

    pub async fn sftp_download_file(
        &self,
        remote_path: &str,
        local_path: &Path,
        progress_tx: Option<mpsc::UnboundedSender<SftpProgress>>,
    ) -> Result<u64> {
        self.sftp_download_file_with_cancel(
            remote_path,
            local_path,
            Uuid::new_v4().to_string(),
            progress_tx,
            None,
        )
        .await
    }

    pub async fn sftp_download_file_with_cancel(
        &self,
        remote_path: &str,
        local_path: &Path,
        transfer_id: String,
        progress_tx: Option<mpsc::UnboundedSender<SftpProgress>>,
        cancel: Option<CancellationToken>,
    ) -> Result<u64> {
        let sftp = self.sftp_session().await?;
        let total = sftp
            .metadata(remote_path)
            .await
            .ok()
            .and_then(|m| m.size)
            .unwrap_or(0);
        let filename = Path::new(remote_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download")
            .to_string();

        let mut remote = sftp
            .open(remote_path)
            .await
            .with_context(|| format!("Failed to open remote file {remote_path}"))?;
        let mut local = tokio::fs::File::create(local_path)
            .await
            .with_context(|| format!("Failed to create local file {}", local_path.display()))?;

        let copied = stream_copy_with_progress(
            &mut remote,
            &mut local,
            TransferKind::Download,
            &transfer_id,
            &filename,
            total,
            progress_tx.as_ref(),
            cancel.as_ref(),
        )
        .await;

        let _ = remote.close().await;
        match copied {
            Ok(transferred) => {
                local.flush().await?;
                emit_done(
                    progress_tx.as_ref(),
                    transfer_id,
                    TransferKind::Download,
                    filename,
                    transferred,
                    transferred.max(total),
                );
                Ok(transferred)
            }
            Err(err) => {
                drop(local);
                let _ = tokio::fs::remove_file(local_path).await;
                Err(err)
            }
        }
    }

    pub async fn sftp_mkdir(&self, path: &str) -> Result<()> {
        let sftp = self.sftp_session().await?;
        sftp.create_dir(path)
            .await
            .with_context(|| format!("Failed to create directory {path}"))?;
        Ok(())
    }

    pub async fn sftp_create_empty_file(&self, path: &str) -> Result<()> {
        let sftp = self.sftp_session().await?;
        let file = sftp
            .create(path)
            .await
            .with_context(|| format!("Failed to create file {path}"))?;
        file.close().await?;
        Ok(())
    }

    pub async fn sftp_rename(&self, old_path: &str, new_path: &str) -> Result<()> {
        let sftp = self.sftp_session().await?;
        sftp.rename(old_path, new_path)
            .await
            .with_context(|| format!("Failed to rename {old_path} to {new_path}"))?;
        Ok(())
    }

    pub async fn sftp_chmod(&self, path: &str, mode: u32) -> Result<()> {
        let sftp = self.sftp_session().await?;
        let attrs = FileAttributes {
            permissions: Some(mode & 0o7777),
            ..Default::default()
        };
        sftp.set_metadata(path, attrs)
            .await
            .with_context(|| format!("Failed to chmod {path} to {:o}", mode & 0o7777))?;
        Ok(())
    }

    pub async fn sftp_delete(&self, path: &str, is_dir: bool) -> Result<()> {
        let sftp = self.sftp_session().await?;
        if is_dir {
            if sftp.remove_dir(path).await.is_err() {
                // Non-empty directories require recursive removal via remote shell
                let cmd = format!("rm -rf {}", posix_quote(path));
                self.exec(&cmd).await?;
            }
        } else {
            sftp.remove_file(path)
                .await
                .with_context(|| format!("Failed to remove file {path}"))?;
        }
        Ok(())
    }

    pub async fn sftp_copy(&self, src: &str, dst: &str) -> Result<()> {
        let cmd = format!("cp -r {} {}", posix_quote(src), posix_quote(dst));
        self.exec(&cmd).await?;
        Ok(())
    }
}

/// Stream a file directly between two independent SSH/SFTP sessions without
/// buffering the entire payload on disk locally.
pub async fn sftp_transfer_between_sessions(
    src: &SshSession,
    src_path: &str,
    dst: &SshSession,
    dst_path: &str,
    transfer_id: String,
    progress_tx: Option<mpsc::UnboundedSender<SftpProgress>>,
    cancel: Option<CancellationToken>,
) -> Result<u64> {
    let src_sftp = src.sftp_session().await?;
    let dst_sftp = dst.sftp_session().await?;
    let total = src_sftp
        .metadata(src_path)
        .await
        .ok()
        .and_then(|m| m.size)
        .unwrap_or(0);
    let filename = Path::new(src_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("remote_copy")
        .to_string();

    let mut src_file = src_sftp
        .open(src_path)
        .await
        .with_context(|| format!("Failed to open source remote file {src_path}"))?;
    let mut dst_file = dst_sftp
        .create(dst_path)
        .await
        .with_context(|| format!("Failed to create destination remote file {dst_path}"))?;

    let copied = stream_copy_with_progress(
        &mut src_file,
        &mut dst_file,
        TransferKind::RemoteCopy,
        &transfer_id,
        &filename,
        total,
        progress_tx.as_ref(),
        cancel.as_ref(),
    )
    .await;

    let _ = src_file.close().await;
    match copied {
        Ok(transferred) => {
            dst_file.flush().await?;
            dst_file.close().await?;
            emit_done(
                progress_tx.as_ref(),
                transfer_id,
                TransferKind::RemoteCopy,
                filename,
                transferred,
                transferred.max(total),
            );
            Ok(transferred)
        }
        Err(err) => {
            let _ = dst_file.close().await;
            let _ = dst_sftp.remove_file(dst_path).await;
            Err(err)
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn stream_copy_with_progress<R, W>(
    reader: &mut R,
    writer: &mut W,
    kind: TransferKind,
    transfer_id: &str,
    filename: &str,
    expected_total: u64,
    progress_tx: Option<&mpsc::UnboundedSender<SftpProgress>>,
    cancel: Option<&CancellationToken>,
) -> Result<u64>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut buf = vec![0u8; TRANSFER_CHUNK_BYTES];
    let mut transferred = 0u64;
    loop {
        if cancel.is_some_and(CancellationToken::is_cancelled) {
            return Err(anyhow!("Transfer cancelled"));
        }
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n]).await?;
        transferred += n as u64;
        if let Some(tx) = progress_tx {
            let _ = tx.send(SftpProgress {
                transfer_id: transfer_id.to_string(),
                kind,
                filename: filename.to_string(),
                transferred,
                total: expected_total.max(transferred).max(1),
            });
        }
    }
    Ok(transferred)
}

fn emit_done(
    progress_tx: Option<&mpsc::UnboundedSender<SftpProgress>>,
    transfer_id: String,
    kind: TransferKind,
    filename: String,
    transferred: u64,
    total: u64,
) {
    if let Some(tx) = progress_tx {
        let _ = tx.send(SftpProgress {
            transfer_id,
            kind,
            filename,
            transferred,
            total,
        });
    }
}
