use crate::ssh::{posix_quote, SshSession};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerSummary {
    pub id: String,
    pub image: String,
    pub name: String,
    pub status: String,
    pub state: String,
}

#[derive(Debug, Clone, Default)]
pub struct ContainerDetails {
    pub raw_json: String,
    pub network_mode: String,
    pub ports: String,
    pub command: String,
    pub restart_policy: String,
    pub mem_usage: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerAction {
    Start,
    Stop,
    Restart,
}

impl ContainerAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
        }
    }
}

pub fn parse_container_list(output: &str) -> Result<Vec<ContainerSummary>> {
    let trimmed = output.trim();
    if trimmed.contains("command not found")
        || trimmed.contains("Cannot connect to the Docker daemon")
        || trimmed.contains("permission denied while trying to connect")
    {
        return Err(anyhow!("{}", trimmed));
    }

    let mut list = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            let id = v["id"].as_str().unwrap_or("").to_string();
            let image = v["image"].as_str().unwrap_or("").to_string();
            let name = v["name"].as_str().unwrap_or("").to_string();
            let status = v["status"].as_str().unwrap_or("").to_string();
            let state = v["state"].as_str().unwrap_or("").to_string();
            if !id.is_empty() || !name.is_empty() {
                list.push(ContainerSummary {
                    id,
                    image,
                    name,
                    status,
                    state,
                });
            }
        }
    }
    Ok(list)
}

pub fn parse_container_inspect(raw_json: &str, mem_usage: String) -> ContainerDetails {
    let parsed: Option<Value> = serde_json::from_str::<Value>(raw_json)
        .ok()
        .and_then(|v| v.as_array().and_then(|a| a.first().cloned()).or(Some(v)));

    let Some(root) = parsed else {
        return ContainerDetails {
            raw_json: raw_json.to_string(),
            network_mode: "—".to_string(),
            ports: "—".to_string(),
            command: "—".to_string(),
            restart_policy: "—".to_string(),
            mem_usage,
        };
    };

    let pretty = serde_json::to_string_pretty(&root).unwrap_or_else(|_| raw_json.to_string());

    let network_mode = root["NetworkSettings"]["Networks"]
        .as_object()
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(", "))
        .filter(|s| !s.is_empty())
        .or_else(|| {
            root["HostConfig"]["NetworkMode"]
                .as_str()
                .map(str::to_string)
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "None".to_string());

    let mut port_mappings = Vec::new();
    if let Some(ports_obj) = root["NetworkSettings"]["Ports"].as_object() {
        for (cport, bindings) in ports_obj {
            if let Some(arr) = bindings.as_array() {
                for b in arr {
                    let hip = b["HostIp"].as_str().filter(|s| !s.is_empty()).unwrap_or("0.0.0.0");
                    let hport = b["HostPort"].as_str().unwrap_or("?");
                    port_mappings.push(format!("{hip}:{hport}->{cport}"));
                }
            } else {
                port_mappings.push(cport.clone());
            }
        }
    }
    let ports = if port_mappings.is_empty() {
        "None".to_string()
    } else {
        port_mappings.join(", ")
    };

    let entry = root["Config"]["Entrypoint"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let cmd = root["Config"]["Cmd"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let command = format!("{entry} {cmd}").trim().to_string();
    let restart_policy = root["HostConfig"]["RestartPolicy"]["Name"]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or("no")
        .to_string();

    ContainerDetails {
        raw_json: pretty,
        network_mode,
        ports,
        command: if command.is_empty() {
            "—".to_string()
        } else {
            command
        },
        restart_policy,
        mem_usage,
    }
}

fn sanitize_container_id(id: &str) -> Result<String> {
    let trimmed = id.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(anyhow!("Invalid container identifier"));
    }
    Ok(trimmed.to_string())
}

fn sanitize_limit(val: &str) -> Result<String> {
    let trimmed = val.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.')
    {
        return Err(anyhow!("Invalid resource limit \"{val}\""));
    }
    Ok(trimmed.to_string())
}

impl SshSession {
    async fn detect_container_runtime(&self) -> Result<&'static str> {
        let out = self
            .exec("if command -v docker >/dev/null 2>&1; then echo docker; elif command -v podman >/dev/null 2>&1; then echo podman; else echo none; fi")
            .await?;
        match out.trim() {
            "docker" => Ok("docker"),
            "podman" => Ok("podman"),
            _ => Err(anyhow!("Docker or Podman is not installed on this server.")),
        }
    }

    pub async fn list_containers(&self) -> Result<(String, Vec<ContainerSummary>)> {
        let bin = self.detect_container_runtime().await?;
        let cmd = format!(
            "{bin} ps -a --format '{{\"id\":\"{{{{.ID}}}}\", \"image\":\"{{{{.Image}}}}\", \"name\":\"{{{{.Names}}}}\", \"status\":\"{{{{.Status}}}}\", \"state\":\"{{{{.State}}}}\"}}' 2>&1"
        );
        let output = self.exec(&cmd).await?;
        let list = parse_container_list(&output)?;
        Ok((output, list))
    }

    pub async fn container_action(&self, id: &str, action: ContainerAction) -> Result<()> {
        let cid = sanitize_container_id(id)?;
        let bin = self.detect_container_runtime().await?;
        let out = self
            .exec(&format!("{bin} {} {cid} 2>&1", action.as_str()))
            .await?;
        if out.to_ascii_lowercase().contains("error") {
            return Err(anyhow!("{}", out.trim()));
        }
        Ok(())
    }

    pub async fn container_inspect(&self, id: &str, is_running: bool) -> Result<ContainerDetails> {
        let cid = sanitize_container_id(id)?;
        let bin = self.detect_container_runtime().await?;
        let inspect_cmd = format!("{bin} inspect {cid} 2>&1");
        let inspect_fut = self.exec(&inspect_cmd);
        let mem_fut = async {
            if is_running {
                self.exec(&format!(
                    "{bin} stats --no-stream --format '{{{{.MemUsage}}}}' {cid} 2>/dev/null"
                ))
                .await
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "—".to_string())
            } else {
                "Offline".to_string()
            }
        };
        let (raw, mem) = tokio::join!(inspect_fut, mem_fut);
        Ok(parse_container_inspect(&raw?, mem))
    }

    pub async fn container_logs(&self, id: &str, tail: u32) -> Result<String> {
        let cid = sanitize_container_id(id)?;
        let bin = self.detect_container_runtime().await?;
        let n = tail.clamp(10, 2000);
        self.exec(&format!("{bin} logs --tail {n} {cid} 2>&1")).await
    }

    pub async fn container_update(
        &self,
        id: &str,
        restart: Option<&str>,
        memory: Option<&str>,
        cpus: Option<&str>,
    ) -> Result<String> {
        let cid = sanitize_container_id(id)?;
        let bin = self.detect_container_runtime().await?;
        let mut args = Vec::new();
        if let Some(r) = restart.map(str::trim).filter(|s| !s.is_empty()) {
            match r {
                "no" | "on-failure" | "always" | "unless-stopped" => {
                    args.push(format!("--restart={r}"));
                }
                _ => return Err(anyhow!("Invalid restart policy \"{r}\"")),
            }
        }
        if let Some(m) = memory.map(str::trim).filter(|s| !s.is_empty()) {
            args.push(format!("--memory={}", sanitize_limit(m)?));
        }
        if let Some(c) = cpus.map(str::trim).filter(|s| !s.is_empty()) {
            args.push(format!("--cpus={}", sanitize_limit(c)?));
        }
        if args.is_empty() {
            return Ok("No settings changed.".to_string());
        }
        self.exec(&format!("{bin} update {} {cid} 2>&1", args.join(" ")))
            .await
    }

    pub async fn compose_action(&self, dir: &str, up: bool) -> Result<String> {
        let trimmed = dir.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Compose directory path is required"));
        }
        let sub = if up { "up -d" } else { "down" };
        let cmd = format!(
            "cd {} && docker compose {sub} 2>&1",
            posix_quote(trimmed)
        );
        self.exec(&cmd).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_docker_ps_and_inspect() {
        let sample = r#"{"id":"abc123def","image":"nginx:alpine","name":"web-proxy","status":"Up 3 hours","state":"running"}
{"id":"zzz999","image":"postgres:16","name":"db","status":"Exited (0)","state":"exited"}"#;
        let list = parse_container_list(sample).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "web-proxy");
        assert_eq!(list[1].state, "exited");

        let inspect_json = r#"[{
            "Config": { "Entrypoint": ["/docker-entrypoint.sh"], "Cmd": ["nginx", "-g", "daemon off;"] },
            "HostConfig": { "RestartPolicy": { "Name": "unless-stopped" } },
            "NetworkSettings": {
                "Networks": { "bridge": {} },
                "Ports": { "80/tcp": [{ "HostIp": "0.0.0.0", "HostPort": "8080" }] }
            }
        }]"#;
        let details = parse_container_inspect(inspect_json, "12.5MiB / 2GiB".into());
        assert_eq!(details.network_mode, "bridge");
        assert_eq!(details.ports, "0.0.0.0:8080->80/tcp");
        assert_eq!(details.restart_policy, "unless-stopped");
        assert!(details.command.contains("nginx"));
    }
}
