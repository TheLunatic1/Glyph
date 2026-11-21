use anyhow::{anyhow, Context, Result};
use directories::BaseDirs;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum McpClientId {
    Antigravity,
    Claude,
    ClaudeCode,
    Cursor,
    VsCode,
}

impl McpClientId {
    pub const ALL: [Self; 5] = [
        Self::Antigravity,
        Self::Claude,
        Self::ClaudeCode,
        Self::Cursor,
        Self::VsCode,
    ];

    pub fn id_str(self) -> &'static str {
        match self {
            Self::Antigravity => "antigravity",
            Self::Claude => "claude",
            Self::ClaudeCode => "claudecode",
            Self::Cursor => "cursor",
            Self::VsCode => "vscode",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Antigravity => "Antigravity IDE",
            Self::Claude => "Claude Desktop",
            Self::ClaudeCode => "Claude Code (CLI)",
            Self::Cursor => "Cursor",
            Self::VsCode => "VS Code",
        }
    }

    pub fn badge_tag(self) -> &'static str {
        match self {
            Self::Antigravity => "AGY",
            Self::Claude => "CD",
            Self::ClaudeCode => "CC",
            Self::Cursor => "CUR",
            Self::VsCode => "VSC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpConfigFormat {
    McpServers,
    VsCode,
    Custom,
}

impl std::fmt::Display for McpConfigFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::McpServers => write!(f, "mcpServers — Standard (Claude, Cursor, Antigravity)"),
            Self::VsCode => write!(f, "mcp.servers — VS Code / GitHub Copilot"),
            Self::Custom => write!(f, "Custom key — Dot-separated path"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpClientStatus {
    Configured,
    NotConfigured,
    NotInstalled,
}

#[derive(Debug, Clone)]
pub struct McpClientInfo {
    pub id: McpClientId,
    pub name: &'static str,
    pub config_path: PathBuf,
    pub format: McpConfigFormat,
    pub status: McpClientStatus,
}

pub fn resolve_mcp_binary_path() -> PathBuf {
    let bin_name = if cfg!(windows) {
        "glyph-mcp.exe"
    } else {
        "glyph-mcp"
    };
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join(bin_name);
            if sibling.exists() {
                return sibling;
            }
        }
    }
    PathBuf::from(bin_name)
}

pub fn client_config_path(id: McpClientId) -> Option<PathBuf> {
    let base = BaseDirs::new()?;
    let home = base.home_dir();
    let config_roaming = base.config_dir();
    let _ = &config_roaming;

    let path = match id {
        McpClientId::Antigravity => home.join(".gemini").join("config").join("mcp_config.json"),
        McpClientId::Claude => {
            #[cfg(target_os = "windows")]
            {
                config_roaming.join("Claude").join("claude_desktop_config.json")
            }
            #[cfg(target_os = "macos")]
            {
                home.join("Library")
                    .join("Application Support")
                    .join("Claude")
                    .join("claude_desktop_config.json")
            }
            #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
            {
                let _ = home;
                config_roaming.join("Claude").join("claude_desktop_config.json")
            }
        }
        McpClientId::ClaudeCode => home.join(".claude.json"),
        McpClientId::Cursor => home.join(".cursor").join("mcp.json"),
        McpClientId::VsCode => {
            #[cfg(target_os = "macos")]
            {
                home.join("Library")
                    .join("Application Support")
                    .join("Code")
                    .join("User")
                    .join("settings.json")
            }
            #[cfg(not(target_os = "macos"))]
            {
                config_roaming.join("Code").join("User").join("settings.json")
            }
        }
    };
    Some(path)
}

pub fn client_format(id: McpClientId) -> McpConfigFormat {
    match id {
        McpClientId::VsCode => McpConfigFormat::VsCode,
        _ => McpConfigFormat::McpServers,
    }
}

pub fn detect_client_status(config_path: &Path, format: McpConfigFormat) -> McpClientStatus {
    if config_path.exists() {
        let configured = fs::read_to_string(config_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .is_some_and(|v| match format {
                McpConfigFormat::VsCode => !v["mcp"]["servers"]["glyph_mcp"].is_null(),
                _ => !v["mcpServers"]["glyph_mcp"].is_null(),
            });
        return if configured {
            McpClientStatus::Configured
        } else {
            McpClientStatus::NotConfigured
        };
    }
    if config_path.parent().is_some_and(Path::exists) {
        McpClientStatus::NotConfigured
    } else {
        McpClientStatus::NotInstalled
    }
}

pub fn scan_mcp_clients() -> Vec<McpClientInfo> {
    McpClientId::ALL
        .iter()
        .filter_map(|&id| {
            let config_path = client_config_path(id)?;
            let format = client_format(id);
            let status = detect_client_status(&config_path, format);
            Some(McpClientInfo {
                id,
                name: id.display_name(),
                config_path,
                format,
                status,
            })
        })
        .collect()
}

pub fn build_config_snippet(
    format: McpConfigFormat,
    custom_key: Option<&str>,
    mcp_bin_path: &Path,
) -> String {
    let cmd = mcp_bin_path.to_string_lossy().replace('\\', "/");
    let mut root = json!({});
    let _ = merge_mcp_entry(&mut root, format, custom_key, &cmd);
    serde_json::to_string_pretty(&root).unwrap_or_default()
}

pub fn auto_install_mcp_client(id: McpClientId, mcp_bin_path: &Path) -> Result<PathBuf> {
    let config_path = client_config_path(id)
        .ok_or_else(|| anyhow!("Unable to resolve user config directory"))?;
    let format = client_format(id);
    write_mcp_config_at(&config_path, format, None, mcp_bin_path)?;
    Ok(config_path)
}

pub fn write_mcp_config_at(
    config_path: &Path,
    format: McpConfigFormat,
    custom_key: Option<&str>,
    mcp_bin_path: &Path,
) -> Result<()> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let mut config: Value = if config_path.exists() {
        let raw = fs::read_to_string(config_path).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if !config.is_object() {
        config = json!({});
    }

    let cmd = mcp_bin_path.to_string_lossy().replace('\\', "/");
    merge_mcp_entry(&mut config, format, custom_key, &cmd)?;

    let serialized = serde_json::to_string_pretty(&config)?;
    fs::write(config_path, serialized)
        .with_context(|| format!("Failed to write MCP config {}", config_path.display()))?;
    Ok(())
}

fn merge_mcp_entry(
    root: &mut Value,
    format: McpConfigFormat,
    custom_key: Option<&str>,
    cmd: &str,
) -> Result<()> {
    let entry = match format {
        McpConfigFormat::VsCode => json!({
            "type": "stdio",
            "command": cmd,
            "args": []
        }),
        McpConfigFormat::McpServers | McpConfigFormat::Custom => json!({
            "command": cmd,
            "args": []
        }),
    };

    let obj = root
        .as_object_mut()
        .ok_or_else(|| anyhow!("Root JSON config must be an object"))?;

    match format {
        McpConfigFormat::VsCode => {
            let mcp = obj
                .entry("mcp")
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
                .ok_or_else(|| anyhow!("'mcp' key is not an object"))?;
            let servers = mcp
                .entry("servers")
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
                .ok_or_else(|| anyhow!("'mcp.servers' key is not an object"))?;
            servers.insert("glyph_mcp".to_string(), entry);
        }
        McpConfigFormat::Custom => {
            let key_path = custom_key
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("mcpServers");
            let mut cursor = obj;
            for part in key_path.split('.').filter(|p| !p.is_empty()) {
                cursor = cursor
                    .entry(part)
                    .or_insert_with(|| Value::Object(Map::new()))
                    .as_object_mut()
                    .ok_or_else(|| anyhow!("'{part}' is not an object in JSON config"))?;
            }
            cursor.insert("glyph_mcp".to_string(), entry);
        }
        McpConfigFormat::McpServers => {
            let servers = obj
                .entry("mcpServers")
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
                .ok_or_else(|| anyhow!("'mcpServers' key is not an object"))?;
            servers.insert("glyph_mcp".to_string(), entry);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn merges_existing_config_for_all_formats() {
        let dir = std::env::temp_dir().join(format!("glyph_mcp_cfg_{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();

        let p1 = dir.join("claude.json");
        fs::write(&p1, r#"{"mcpServers":{"other":{"command":"foo"}}}"#).unwrap();
        write_mcp_config_at(
            &p1,
            McpConfigFormat::McpServers,
            None,
            Path::new("C:\\bin\\glyph-mcp.exe"),
        )
        .unwrap();
        let v1: Value = serde_json::from_str(&fs::read_to_string(&p1).unwrap()).unwrap();
        assert_eq!(v1["mcpServers"]["other"]["command"], "foo");
        assert_eq!(v1["mcpServers"]["glyph_mcp"]["command"], "C:/bin/glyph-mcp.exe");
        assert_eq!(
            detect_client_status(&p1, McpConfigFormat::McpServers),
            McpClientStatus::Configured
        );

        let p2 = dir.join("vscode.json");
        write_mcp_config_at(
            &p2,
            McpConfigFormat::VsCode,
            None,
            Path::new("/usr/local/bin/glyph-mcp"),
        )
        .unwrap();
        let v2: Value = serde_json::from_str(&fs::read_to_string(&p2).unwrap()).unwrap();
        assert_eq!(v2["mcp"]["servers"]["glyph_mcp"]["type"], "stdio");

        let p3 = dir.join("custom.json");
        write_mcp_config_at(
            &p3,
            McpConfigFormat::Custom,
            Some("aiTool.mcpServers"),
            Path::new("/opt/glyph-mcp"),
        )
        .unwrap();
        let v3: Value = serde_json::from_str(&fs::read_to_string(&p3).unwrap()).unwrap();
        assert_eq!(
            v3["aiTool"]["mcpServers"]["glyph_mcp"]["command"],
            "/opt/glyph-mcp"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
