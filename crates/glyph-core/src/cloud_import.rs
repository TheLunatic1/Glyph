use crate::models::{AuthMethod, ServerConfigInput};
use crate::updater::ensure_rustls_ring_provider;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    DigitalOcean,
    Hetzner,
    Tailscale,
}

impl CloudProvider {
    pub const ALL: [Self; 3] = [Self::DigitalOcean, Self::Hetzner, Self::Tailscale];
}

impl std::fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DigitalOcean => write!(f, "DigitalOcean"),
            Self::Hetzner => write!(f, "Hetzner Cloud"),
            Self::Tailscale => write!(f, "Tailscale"),
        }
    }
}

pub async fn fetch_cloud_hosts(
    provider: CloudProvider,
    api_token: &str,
    tailnet: Option<&str>,
) -> Result<Vec<ServerConfigInput>> {
    ensure_rustls_ring_provider();
    let token = api_token.trim();
    if token.is_empty() {
        return Err(anyhow!("API token is required"));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Glyph-Desktop-v3")
        .build()?;

    match provider {
        CloudProvider::DigitalOcean => {
            #[derive(Deserialize)]
            struct DoResp {
                droplets: Vec<DoDroplet>,
            }
            #[derive(Deserialize)]
            struct DoDroplet {
                name: String,
                tags: Option<Vec<String>>,
                networks: DoNetworks,
            }
            #[derive(Deserialize)]
            struct DoNetworks {
                v4: Option<Vec<DoV4>>,
            }
            #[derive(Deserialize)]
            struct DoV4 {
                ip_address: String,
                #[serde(rename = "type")]
                kind: String,
            }

            let resp: DoResp = client
                .get("https://api.digitalocean.com/v2/droplets?per_page=200")
                .bearer_auth(token)
                .send()
                .await
                .context("DigitalOcean request failed")?
                .error_for_status()
                .context("DigitalOcean API rejected token or request")?
                .json()
                .await?;

            Ok(resp
                .droplets
                .into_iter()
                .filter_map(|d| {
                    let v4 = d.networks.v4?;
                    let ip = v4
                        .iter()
                        .find(|n| n.kind == "public")
                        .or_else(|| v4.first())?
                        .ip_address
                        .clone();
                    let mut tags = d.tags.unwrap_or_default();
                    tags.push("digitalocean".to_string());
                    Some(ServerConfigInput {
                        name: d.name,
                        host: ip,
                        username: "root".to_string(),
                        port: 22,
                        auth_method: AuthMethod::Agent,
                        tags,
                        ..Default::default()
                    })
                })
                .collect())
        }
        CloudProvider::Hetzner => {
            #[derive(Deserialize)]
            struct HzResp {
                servers: Vec<HzServer>,
            }
            #[derive(Deserialize)]
            struct HzServer {
                name: String,
                public_net: HzPubNet,
            }
            #[derive(Deserialize)]
            struct HzPubNet {
                ipv4: Option<HzIpv4>,
            }
            #[derive(Deserialize)]
            struct HzIpv4 {
                ip: String,
            }

            let resp: HzResp = client
                .get("https://api.hetzner.cloud/v1/servers?per_page=50")
                .bearer_auth(token)
                .send()
                .await
                .context("Hetzner request failed")?
                .error_for_status()
                .context("Hetzner Cloud API rejected token or request")?
                .json()
                .await?;

            Ok(resp
                .servers
                .into_iter()
                .filter_map(|s| {
                    let ip = s.public_net.ipv4?.ip;
                    Some(ServerConfigInput {
                        name: s.name,
                        host: ip,
                        username: "root".to_string(),
                        port: 22,
                        auth_method: AuthMethod::Agent,
                        tags: vec!["hetzner".to_string()],
                        ..Default::default()
                    })
                })
                .collect())
        }
        CloudProvider::Tailscale => {
            #[derive(Deserialize)]
            struct TsResp {
                devices: Vec<TsDevice>,
            }
            #[derive(Deserialize)]
            struct TsDevice {
                hostname: Option<String>,
                name: Option<String>,
                addresses: Vec<String>,
                os: Option<String>,
            }

            let net = tailnet
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("-");
            let url = format!("https://api.tailscale.com/api/v2/tailnet/{net}/devices");
            let resp: TsResp = client
                .get(&url)
                .bearer_auth(token)
                .send()
                .await
                .context("Tailscale request failed")?
                .error_for_status()
                .context("Tailscale API rejected token or tailnet")?
                .json()
                .await?;

            Ok(resp
                .devices
                .into_iter()
                .filter_map(|d| {
                    let ip = d
                        .addresses
                        .iter()
                        .find(|a| !a.contains(':'))
                        .or_else(|| d.addresses.first())?
                        .clone();
                    let label = d
                        .hostname
                        .filter(|s| !s.is_empty())
                        .or(d.name)
                        .unwrap_or_else(|| ip.clone());
                    let mut tags = vec!["tailscale".to_string()];
                    if let Some(os) = d.os.filter(|s| !s.is_empty()) {
                        tags.push(os);
                    }
                    Some(ServerConfigInput {
                        name: label,
                        host: ip,
                        username: "root".to_string(),
                        port: 22,
                        auth_method: AuthMethod::Agent,
                        tags,
                        ..Default::default()
                    })
                })
                .collect())
        }
    }
}
