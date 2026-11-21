use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::cmp::Ordering;
use std::sync::Once;

static RUSTLS_INIT: Once = Once::new();

pub fn ensure_rustls_ring_provider() {
    RUSTLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub version: String,
    pub html_url: String,
    pub asset_name: Option<String>,
    pub download_url: Option<String>,
    pub asset_size: Option<u64>,
    pub published_at: Option<String>,
}

fn parse_semver(raw: &str) -> Option<([u64; 3], bool, String)> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    let (core, pre) = match trimmed.split_once('-') {
        Some((c, p)) => (c, Some(p.to_string())),
        None => (trimmed, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some(([major, minor, patch], pre.is_none(), pre.unwrap_or_default()))
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let Some((c_nums, c_stable, c_pre)) = parse_semver(current) else {
        return false;
    };
    let Some((l_nums, l_stable, l_pre)) = parse_semver(latest) else {
        return false;
    };
    match l_nums.cmp(&c_nums) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => match (l_stable, c_stable) {
            (true, false) => true,
            (false, true) => false,
            _ => l_pre > c_pre,
        },
    }
}

pub async fn fetch_latest_release(
    repo_slug: &str,
    preferred_suffix: Option<&str>,
) -> Result<ReleaseInfo> {
    ensure_rustls_ring_provider();
    let url = format!("https://api.github.com/repos/{repo_slug}/releases/latest");
    let client = reqwest::Client::builder()
        .user_agent("Glyph-Desktop")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .with_context(|| format!("Failed to reach GitHub releases at {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("GitHub API returned HTTP {}", resp.status()));
    }
    let json: Value = resp.json().await?;
    let tag_name = json["tag_name"]
        .as_str()
        .unwrap_or("v0.0.0")
        .to_string();
    let version = if tag_name.starts_with('v') {
        tag_name.clone()
    } else {
        format!("v{tag_name}")
    };
    let html_url = json["html_url"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("https://github.com/{repo_slug}/releases/latest"));

    let assets = json["assets"].as_array();
    let picked = assets.and_then(|arr| {
        if let Some(suf) = preferred_suffix {
            arr.iter()
                .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with(suf)))
                .or_else(|| arr.first())
        } else {
            arr.first()
        }
    });

    Ok(ReleaseInfo {
        tag_name,
        version,
        html_url,
        asset_name: picked.and_then(|a| a["name"].as_str()).map(str::to_string),
        download_url: picked
            .and_then(|a| a["browser_download_url"].as_str())
            .map(str::to_string),
        asset_size: picked.and_then(|a| a["size"].as_u64()),
        published_at: json["published_at"].as_str().map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison_handles_prerelease_and_v_prefix() {
        assert!(is_newer_version("3.0.0-alpha.1", "v3.0.0"));
        assert!(is_newer_version("3.0.0-alpha.1", "v3.0.0-alpha.2"));
        assert!(is_newer_version("v2.7.6", "3.0.0"));
        assert!(!is_newer_version("3.0.0", "v3.0.0-alpha.9"));
        assert!(!is_newer_version("v2.7.6", "2.7.6"));
    }
}
