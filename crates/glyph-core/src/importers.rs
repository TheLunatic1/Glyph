use crate::models::{AuthMethod, ServerConfigInput, ServerSummary};
use directories::BaseDirs;
use std::path::PathBuf;

pub fn default_ssh_config_path() -> Option<PathBuf> {
    if let Some(base) = BaseDirs::new() {
        return Some(base.home_dir().join(".ssh").join("config"));
    }
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(|h| PathBuf::from(h).join(".ssh").join("config"))
}

#[derive(Default, Clone)]
struct SshBlock {
    patterns: Vec<String>,
    hostname: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    identity_file: Option<String>,
    forward_agent: Option<bool>,
    keepalive_secs: Option<u16>,
}

/// Parses an OpenSSH `~/.ssh/config` document into Glyph `ServerConfigInput` items.
/// Wildcard `Host *` entries contribute default values to concrete blocks that omit them.
pub fn parse_ssh_config(content: &str) -> Vec<ServerConfigInput> {
    let mut blocks: Vec<SshBlock> = Vec::new();
    let mut current: Option<SshBlock> = None;

    for raw_line in content.lines() {
        let stripped = raw_line.split('#').next().unwrap_or("").trim();
        if stripped.is_empty() {
            continue;
        }
        let mut parts = stripped.splitn(2, |c: char| c.is_whitespace() || c == '=');
        let key = parts.next().unwrap_or("").trim().to_ascii_lowercase();
        let val = parts
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches('=')
            .trim();
        if val.is_empty() {
            continue;
        }

        if key == "host" {
            if let Some(prev) = current.take() {
                blocks.push(prev);
            }
            let patterns = val
                .split_whitespace()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            current = Some(SshBlock {
                patterns,
                ..Default::default()
            });
            continue;
        }

        let Some(blk) = current.as_mut() else {
            continue;
        };
        let clean_val = val.trim_matches('"');
        match key.as_str() {
            "hostname" => blk.hostname = Some(clean_val.to_string()),
            "user" => blk.user = Some(clean_val.to_string()),
            "port" => {
                if let Ok(p) = clean_val.parse::<u16>() {
                    if p > 0 {
                        blk.port = Some(p);
                    }
                }
            }
            "identityfile" => {
                if blk.identity_file.is_none() {
                    blk.identity_file = Some(expand_tilde(clean_val));
                }
            }
            "forwardagent" => {
                blk.forward_agent = Some(clean_val.eq_ignore_ascii_case("yes") || clean_val == "true");
            }
            "serveraliveinterval" => {
                if let Ok(sec) = clean_val.parse::<u16>() {
                    blk.keepalive_secs = Some(sec);
                }
            }
            _ => {}
        }
    }
    if let Some(last) = current {
        blocks.push(last);
    }

    // Gather `Host *` global fallback defaults
    let wildcard_defaults: SshBlock = blocks
        .iter()
        .find(|b| b.patterns.iter().any(|p| p == "*"))
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for b in blocks {
        for pat in &b.patterns {
            if pat.contains('*') || pat.contains('?') || pat.starts_with('!') {
                continue;
            }
            let host = b
                .hostname
                .clone()
                .or_else(|| wildcard_defaults.hostname.clone())
                .unwrap_or_else(|| pat.clone());
            let username = b
                .user
                .clone()
                .or_else(|| wildcard_defaults.user.clone())
                .unwrap_or_else(|| "root".to_string());
            let port = b.port.or(wildcard_defaults.port).unwrap_or(22);
            let key_path = b
                .identity_file
                .clone()
                .or_else(|| wildcard_defaults.identity_file.clone())
                .unwrap_or_default();
            let forward_agent = b
                .forward_agent
                .or(wildcard_defaults.forward_agent)
                .unwrap_or(false);
            let keepalive_secs = b
                .keepalive_secs
                .or(wildcard_defaults.keepalive_secs)
                .unwrap_or(15);

            out.push(ServerConfigInput {
                name: pat.clone(),
                host,
                username,
                port,
                private_key_path: key_path.clone(),
                auth_method: if key_path.is_empty() {
                    AuthMethod::Agent
                } else {
                    AuthMethod::KeyFile
                },
                forward_agent,
                keepalive_secs,
                tags: vec!["ssh-config".to_string()],
                ..Default::default()
            });
        }
    }
    out
}

pub fn export_ssh_config(servers: &[ServerSummary]) -> String {
    let mut out = String::from("# Exported by Glyph v3\n\n");
    for s in servers {
        let alias_raw = if s.name.trim().is_empty() {
            &s.host
        } else {
            &s.name
        };
        let alias: String = alias_raw
            .chars()
            .map(|c| if c.is_whitespace() { '-' } else { c })
            .collect();
        out.push_str(&format!("Host {alias}\n"));
        out.push_str(&format!("    HostName {}\n", s.host));
        out.push_str(&format!("    Port {}\n", s.port));
        if !s.username.is_empty() {
            out.push_str(&format!("    User {}\n", s.username));
        }
        if !s.private_key_path.is_empty() {
            out.push_str(&format!("    IdentityFile {}\n", s.private_key_path));
        }
        if s.forward_agent {
            out.push_str("    ForwardAgent yes\n");
        }
        if s.keepalive_secs > 0 {
            out.push_str(&format!("    ServerAliveInterval {}\n", s.keepalive_secs));
        }
        out.push('\n');
    }
    out
}

/// Parses exported Windows PuTTY registry `.reg` files.
pub fn parse_putty_reg(content: &str) -> Vec<ServerConfigInput> {
    #[derive(Default)]
    struct PuttySess {
        name: String,
        host: String,
        port: u16,
        user: String,
    }

    let mut sessions = Vec::new();
    let mut cur: Option<PuttySess> = None;

    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(prev) = cur.take() {
                if !prev.host.is_empty() {
                    sessions.push(prev);
                }
            }
            let section = &line[1..line.len() - 1];
            if let Some(pos) = section.rfind("\\Sessions\\") {
                let raw_name = &section[pos + "\\Sessions\\".len()..];
                let decoded = decode_putty_name(raw_name);
                if decoded != "Default Settings" {
                    cur = Some(PuttySess {
                        name: decoded,
                        port: 22,
                        ..Default::default()
                    });
                }
            }
            continue;
        }

        let Some(sess) = cur.as_mut() else {
            continue;
        };
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim().trim_matches('"');
        let val = v.trim();
        match key {
            "HostName" => sess.host = val.trim_matches('"').to_string(),
            "UserName" => sess.user = val.trim_matches('"').to_string(),
            "PortNumber" => {
                if let Some(hex) = val.strip_prefix("dword:") {
                    if let Ok(p) = u32::from_str_radix(hex, 16) {
                        if (1..=65535).contains(&p) {
                            sess.port = p as u16;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(last) = cur {
        if !last.host.is_empty() {
            sessions.push(last);
        }
    }

    sessions
        .into_iter()
        .map(|s| ServerConfigInput {
            name: if s.name.is_empty() {
                s.host.clone()
            } else {
                s.name
            },
            host: s.host,
            username: if s.user.is_empty() {
                "root".to_string()
            } else {
                s.user
            },
            port: s.port,
            tags: vec!["putty".to_string()],
            ..Default::default()
        })
        .collect()
}

/// Parses CSV rows: `name,host,port,username,tags` (header row optional).
pub fn parse_hosts_csv(content: &str) -> Vec<ServerConfigInput> {
    let mut out = Vec::new();
    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split(',').map(|c| c.trim().trim_matches('"')).collect();
        if idx == 0 && cols.first().is_some_and(|c| c.eq_ignore_ascii_case("name")) {
            continue;
        }
        let host = cols.get(1).copied().unwrap_or("");
        if host.is_empty() {
            continue;
        }
        let name = cols.first().copied().filter(|s| !s.is_empty()).unwrap_or(host);
        let port = cols
            .get(2)
            .and_then(|p| p.parse::<u16>().ok())
            .filter(|&p| p > 0)
            .unwrap_or(22);
        let username = cols
            .get(3)
            .copied()
            .filter(|s| !s.is_empty())
            .unwrap_or("root");
        let tags = cols
            .get(4)
            .map(|t| {
                t.split([';', '|'])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        out.push(ServerConfigInput {
            name: name.to_string(),
            host: host.to_string(),
            port,
            username: username.to_string(),
            tags,
            ..Default::default()
        });
    }
    out
}

fn decode_putty_name(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(hex_str, 16) {
                    out.push(val);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
        if let Some(base) = BaseDirs::new() {
            return base.home_dir().join(rest).display().to_string();
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_config_parse_and_export_roundtrip() {
        let raw = r#"
Host *
    User ops
    ServerAliveInterval 30

Host web-prod
    HostName 10.20.30.40
    Port 2222
    IdentityFile ~/.ssh/prod_ed25519
    ForwardAgent yes
"#;
        let parsed = parse_ssh_config(raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "web-prod");
        assert_eq!(parsed[0].host, "10.20.30.40");
        assert_eq!(parsed[0].port, 2222);
        assert_eq!(parsed[0].username, "ops");
        assert!(parsed[0].forward_agent);
        assert_eq!(parsed[0].keepalive_secs, 30);

        let sum = ServerSummary {
            id: "1".into(),
            name: parsed[0].name.clone(),
            host: parsed[0].host.clone(),
            username: parsed[0].username.clone(),
            port: parsed[0].port,
            os: None,
            zerotier: String::new(),
            private_key_path: parsed[0].private_key_path.clone(),
            auth_method: parsed[0].auth_method,
            ssh_key_id: None,
            proxy_jump_id: None,
            tags: vec![],
            has_password: false,
            group_id: None,
            identity_id: None,
            forward_agent: parsed[0].forward_agent,
            keepalive_secs: parsed[0].keepalive_secs,
            startup_command: String::new(),
            env_vars: vec![],
            port_knock: vec![],
            saved_tunnels: vec![],
            auto_log: false,
        };
        let exported = export_ssh_config(&[sum]);
        assert!(exported.contains("Host web-prod"));
        assert!(exported.contains("HostName 10.20.30.40"));
        assert!(exported.contains("ForwardAgent yes"));
    }

    #[test]
    fn putty_reg_and_csv_parsers_work() {
        let reg = r#"Windows Registry Editor Version 5.00
[HKEY_CURRENT_USER\Software\SimonTatham\PuTTY\Sessions\My%20Prod%20Box]
"HostName"="192.168.50.10"
"UserName"="deploy"
"PortNumber"=dword:000008ae
"#;
        let parsed = parse_putty_reg(reg);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "My Prod Box");
        assert_eq!(parsed[0].host, "192.168.50.10");
        assert_eq!(parsed[0].username, "deploy");
        assert_eq!(parsed[0].port, 2222);

        let csv = "name,host,port,username,tags\ndb-1,10.0.0.5,5422,postgres,prod;db\n";
        let csv_hosts = parse_hosts_csv(csv);
        assert_eq!(csv_hosts.len(), 1);
        assert_eq!(csv_hosts[0].name, "db-1");
        assert_eq!(csv_hosts[0].port, 5422);
        assert_eq!(csv_hosts[0].tags, vec!["prod", "db"]);
    }
}
