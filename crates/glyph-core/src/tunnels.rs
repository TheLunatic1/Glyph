pub use crate::models::TunnelDirection;
use crate::ssh::SshSession;
use anyhow::{anyhow, Context, Result};
use russh::client;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelInfo {
    pub id: String,
    pub direction: TunnelDirection,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub protocol: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteTunnelTarget {
    pub remote_bind_addr: String,
    pub remote_port: u16,
    pub local_host: String,
    pub local_port: u16,
}

pub(crate) struct ActiveTunnel {
    pub info: TunnelInfo,
    pub cancel: CancellationToken,
    pub task: Option<JoinHandle<()>>,
}

impl SshSession {
    pub async fn start_local_tunnel(
        &self,
        local_port: u16,
        remote_host: String,
        remote_port: u16,
        protocol: String,
    ) -> Result<TunnelInfo> {
        let host = remote_host.trim().to_string();
        if host.is_empty() || remote_port == 0 {
            return Err(anyhow!("Remote host and non-zero remote port are required"));
        }

        let listener = TcpListener::bind(("127.0.0.1", local_port))
            .await
            .with_context(|| format!("Failed to bind local port {local_port}"))?;
        let bound_port = listener.local_addr()?.port();
        let id = format!("L:{bound_port}");

        {
            let map = self.inner.tunnels.lock().await;
            if map.contains_key(&id) {
                return Err(anyhow!("Tunnel already active on local port {bound_port}"));
            }
        }

        let proto = if protocol.trim().is_empty() {
            "tcp".to_string()
        } else {
            protocol.trim().to_ascii_lowercase()
        };

        let info = TunnelInfo {
            id: id.clone(),
            direction: TunnelDirection::Local,
            local_port: bound_port,
            remote_host: host.clone(),
            remote_port,
            protocol: proto,
        };

        let cancel = CancellationToken::new();
        let cancel_loop = cancel.clone();
        let handle_arc = self.inner.handle.clone();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_loop.cancelled() => break,
                    accepted = listener.accept() => {
                        let Ok((mut client_sock, _)) = accepted else {
                            break;
                        };
                        let handle_clone = handle_arc.clone();
                        let rh = host.clone();
                        let rp = remote_port;
                        let conn_cancel = cancel_loop.clone();
                        tokio::spawn(async move {
                            let ch_res = {
                                let guard = handle_clone.lock().await;
                                match guard.as_ref() {
                                    Some(h) => h.channel_open_direct_tcpip(&rh, rp as u32, "127.0.0.1", 0).await,
                                    None => return,
                                }
                            };
                            if let Ok(channel) = ch_res {
                                let mut ssh_stream = channel.into_stream();
                                tokio::select! {
                                    _ = conn_cancel.cancelled() => {},
                                    _ = tokio::io::copy_bidirectional(&mut client_sock, &mut ssh_stream) => {},
                                }
                            }
                        });
                    }
                }
            }
        });

        self.inner.tunnels.lock().await.insert(
            id,
            ActiveTunnel {
                info: info.clone(),
                cancel,
                task: Some(task),
            },
        );

        Ok(info)
    }

    pub async fn start_remote_tunnel(
        &self,
        remote_bind_addr: String,
        remote_port: u16,
        local_host: String,
        local_port: u16,
    ) -> Result<TunnelInfo> {
        let bind_addr = if remote_bind_addr.trim().is_empty() {
            "127.0.0.1".to_string()
        } else {
            remote_bind_addr.trim().to_string()
        };
        let lhost = if local_host.trim().is_empty() {
            "127.0.0.1".to_string()
        } else {
            local_host.trim().to_string()
        };
        if local_port == 0 {
            return Err(anyhow!("Local target port must be non-zero"));
        }

        let assigned = {
            let guard = self.inner.handle.lock().await;
            let handle = guard.as_ref().ok_or_else(|| anyhow!("Not connected"))?;
            handle
                .tcpip_forward(&bind_addr, remote_port as u32)
                .await
                .with_context(|| {
                    format!("Remote SSH forwarding rejected for {bind_addr}:{remote_port}")
                })?
        };
        let actual_remote_port = if assigned == 0 {
            remote_port
        } else {
            assigned as u16
        };
        let id = format!("R:{actual_remote_port}");

        self.inner.remote_tunnels.lock().await.insert(
            actual_remote_port,
            RemoteTunnelTarget {
                remote_bind_addr: bind_addr.clone(),
                remote_port: actual_remote_port,
                local_host: lhost.clone(),
                local_port,
            },
        );

        let info = TunnelInfo {
            id: id.clone(),
            direction: TunnelDirection::Remote,
            local_port,
            remote_host: format!("{bind_addr} -> {lhost}"),
            remote_port: actual_remote_port,
            protocol: "tcp".to_string(),
        };

        self.inner.tunnels.lock().await.insert(
            id,
            ActiveTunnel {
                info: info.clone(),
                cancel: CancellationToken::new(),
                task: None,
            },
        );

        Ok(info)
    }

    /// Starts an RFC 1928 Dynamic SOCKS5 (`-D`) proxy server over SSH `direct-tcpip`.
    pub async fn start_dynamic_socks5_tunnel(&self, local_port: u16) -> Result<TunnelInfo> {
        let listener = TcpListener::bind(("127.0.0.1", local_port))
            .await
            .with_context(|| format!("Failed to bind SOCKS5 port {local_port}"))?;
        let bound_port = listener.local_addr()?.port();
        let id = format!("D:{bound_port}");

        {
            let map = self.inner.tunnels.lock().await;
            if map.contains_key(&id) {
                return Err(anyhow!("SOCKS5 proxy already active on port {bound_port}"));
            }
        }

        let info = TunnelInfo {
            id: id.clone(),
            direction: TunnelDirection::Dynamic,
            local_port: bound_port,
            remote_host: "SOCKS5 Dynamic".to_string(),
            remote_port: 0,
            protocol: "socks5".to_string(),
        };

        let cancel = CancellationToken::new();
        let cancel_loop = cancel.clone();
        let handle_arc = self.inner.handle.clone();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_loop.cancelled() => break,
                    accepted = listener.accept() => {
                        let Ok((client_sock, _)) = accepted else {
                            break;
                        };
                        let handle_clone = Arc::clone(&handle_arc);
                        let conn_cancel = cancel_loop.clone();
                        tokio::spawn(async move {
                            let _ = handle_socks5_client(client_sock, handle_clone, conn_cancel).await;
                        });
                    }
                }
            }
        });

        self.inner.tunnels.lock().await.insert(
            id,
            ActiveTunnel {
                info: info.clone(),
                cancel,
                task: Some(task),
            },
        );

        Ok(info)
    }

    pub async fn stop_tunnel_by_id(&self, id: &str) -> Result<()> {
        let removed = self.inner.tunnels.lock().await.remove(id);
        let Some(active) = removed else {
            return Err(anyhow!("Tunnel \"{id}\" is not active"));
        };

        active.cancel.cancel();
        if let Some(task) = active.task {
            task.abort();
        }

        if active.info.direction == TunnelDirection::Remote {
            if let Some(target) = self
                .inner
                .remote_tunnels
                .lock()
                .await
                .remove(&active.info.remote_port)
            {
                if let Some(handle) = self.inner.handle.lock().await.as_ref() {
                    let _ = handle
                        .cancel_tcpip_forward(&target.remote_bind_addr, target.remote_port as u32)
                        .await;
                }
            }
        }
        Ok(())
    }

    pub async fn stop_tunnel(&self, local_port: u16) -> Result<()> {
        let local_id = format!("L:{local_port}");
        if self.inner.tunnels.lock().await.contains_key(&local_id) {
            return self.stop_tunnel_by_id(&local_id).await;
        }
        let dyn_id = format!("D:{local_port}");
        if self.inner.tunnels.lock().await.contains_key(&dyn_id) {
            return self.stop_tunnel_by_id(&dyn_id).await;
        }
        let matching = {
            let map = self.inner.tunnels.lock().await;
            map.values()
                .find(|t| t.info.local_port == local_port || t.info.remote_port == local_port)
                .map(|t| t.info.id.clone())
        };
        if let Some(id) = matching {
            return self.stop_tunnel_by_id(&id).await;
        }
        Err(anyhow!("No active tunnel found on port {local_port}"))
    }

    pub async fn list_tunnels(&self) -> Vec<TunnelInfo> {
        let map = self.inner.tunnels.lock().await;
        let mut list: Vec<TunnelInfo> = map.values().map(|t| t.info.clone()).collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }
}

async fn handle_socks5_client(
    mut sock: tokio::net::TcpStream,
    handle_arc: Arc<Mutex<Option<client::Handle<crate::ssh::GlyphHandler>>>>,
    cancel: CancellationToken,
) -> Result<()> {
    // 1. Version + auth methods negotiation
    let mut head = [0u8; 2];
    sock.read_exact(&mut head).await?;
    if head[0] != 0x05 {
        return Err(anyhow!("Unsupported SOCKS version"));
    }
    let nmethods = head[1] as usize;
    let mut methods = vec![0u8; nmethods];
    sock.read_exact(&mut methods).await?;
    if !methods.contains(&0x00) {
        let _ = sock.write_all(&[0x05, 0xFF]).await;
        return Err(anyhow!("Client does not offer NO AUTH"));
    }
    sock.write_all(&[0x05, 0x00]).await?;

    // 2. Request header: VER CMD RSV ATYP
    let mut req = [0u8; 4];
    sock.read_exact(&mut req).await?;
    if req[0] != 0x05 {
        return Err(anyhow!("Invalid SOCKS5 request version"));
    }
    let cmd = req[1];
    let atyp = req[3];
    if cmd != 0x01 {
        let _ = sock
            .write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
            .await;
        return Err(anyhow!("Only SOCKS5 CONNECT is supported"));
    }

    let target_host = match atyp {
        0x01 => {
            let mut octets = [0u8; 4];
            sock.read_exact(&mut octets).await?;
            Ipv4Addr::from(octets).to_string()
        }
        0x03 => {
            let len = sock.read_u8().await? as usize;
            let mut domain = vec![0u8; len];
            sock.read_exact(&mut domain).await?;
            String::from_utf8(domain).context("Invalid SOCKS5 domain")?
        }
        0x04 => {
            let mut octets = [0u8; 16];
            sock.read_exact(&mut octets).await?;
            Ipv6Addr::from(octets).to_string()
        }
        _ => {
            let _ = sock
                .write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await;
            return Err(anyhow!("Unsupported SOCKS5 address type {atyp}"));
        }
    };

    let mut port_buf = [0u8; 2];
    sock.read_exact(&mut port_buf).await?;
    let target_port = u16::from_be_bytes(port_buf);

    let ch_res = {
        let guard = handle_arc.lock().await;
        match guard.as_ref() {
            Some(h) => {
                h.channel_open_direct_tcpip(&target_host, target_port as u32, "127.0.0.1", 0)
                    .await
            }
            None => return Err(anyhow!("SSH session closed")),
        }
    };

    let channel = match ch_res {
        Ok(c) => c,
        Err(e) => {
            let _ = sock
                .write_all(&[0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await;
            return Err(anyhow!("SOCKS5 SSH forward failed: {e}"));
        }
    };

    sock.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await?;
    let mut ssh_stream = channel.into_stream();
    tokio::select! {
        _ = cancel.cancelled() => {},
        _ = tokio::io::copy_bidirectional(&mut sock, &mut ssh_stream) => {},
    }
    Ok(())
}
