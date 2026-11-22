use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

static RUSTLS_INIT: Once = Once::new();

/// Ensures the process-wide `rustls` crypto provider uses `ring`.
pub fn ensure_rustls_ring_provider() {
    RUSTLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Metadata describing an individual downloadable asset attached to a GitHub release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
}

/// Normalized metadata for a GitHub release and its downloadable artifacts.
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub version: String,
    pub html_url: String,
    pub asset_name: Option<String>,
    pub download_url: Option<String>,
    pub asset_size: Option<u64>,
    pub published_at: Option<String>,
    pub release_notes: String,
    pub checksums_url: Option<String>,
    pub assets: Vec<ReleaseAsset>,
}

/// Platform installer strategy selected for a downloaded desktop update artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateInstallKind {
    WindowsInstaller,
    WindowsStandalone,
    WindowsZipBundle,
    MacOsAppZip,
    MacOsDmgOpen,
    LinuxAppImage,
    LinuxTarball,
}

/// Streaming download progress snapshot emitted while fetching a release binary.
#[derive(Debug, Clone)]
pub struct UpdateDownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: u8,
    pub bytes_per_sec: f64,
}

/// Completed, optionally checksum-verified release download ready for installation.
#[derive(Debug, Clone)]
pub struct DownloadedUpdate {
    pub version: String,
    pub tag_name: String,
    pub asset_name: String,
    pub file_path: PathBuf,
    pub sha256_hex: String,
    pub sha256_verified: bool,
    pub install_kind: UpdateInstallKind,
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

/// Returns `true` if `latest` represents a strictly newer semver than `current`.
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

/// Fetches the latest GitHub release for `repo_slug` from `https://api.github.com`.
pub async fn fetch_latest_release(
    repo_slug: &str,
    preferred_suffix: Option<&str>,
) -> Result<ReleaseInfo> {
    fetch_latest_release_from_base("https://api.github.com", repo_slug, preferred_suffix).await
}

/// Fetches the latest release JSON from a configurable GitHub-compatible API base URL.
pub async fn fetch_latest_release_from_base(
    api_base: &str,
    repo_slug: &str,
    preferred_suffix: Option<&str>,
) -> Result<ReleaseInfo> {
    ensure_rustls_ring_provider();
    let base = api_base.trim_end_matches('/');
    let url = format!("{base}/repos/{repo_slug}/releases/latest");
    let client = reqwest::Client::builder()
        .user_agent("Glyph-Desktop")
        .timeout(Duration::from_secs(15))
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
    let tag_name = json["tag_name"].as_str().unwrap_or("v0.0.0").to_string();
    let version = if tag_name.starts_with('v') {
        tag_name.clone()
    } else {
        format!("v{tag_name}")
    };
    let html_url = json["html_url"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("https://github.com/{repo_slug}/releases/latest"));
    let release_notes = json["body"].as_str().unwrap_or("").trim().to_string();

    let assets: Vec<ReleaseAsset> = json["assets"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let name = a["name"].as_str()?.trim().to_string();
                    let download_url = a["browser_download_url"].as_str()?.trim().to_string();
                    if name.is_empty() || download_url.is_empty() {
                        return None;
                    }
                    Some(ReleaseAsset {
                        name,
                        download_url,
                        size: a["size"].as_u64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let checksums_url = assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("SHA256SUMS.txt"))
        .map(|a| a.download_url.clone());

    let picked = if let Some(suf) = preferred_suffix {
        assets
            .iter()
            .find(|a| a.name.ends_with(suf))
            .or_else(|| assets.first())
            .cloned()
    } else {
        select_desktop_asset_from_slice(&assets)
            .map(|(a, _)| a)
            .or_else(|| assets.first().cloned())
    };

    Ok(ReleaseInfo {
        tag_name,
        version,
        html_url,
        asset_name: picked.as_ref().map(|a| a.name.clone()),
        download_url: picked.as_ref().map(|a| a.download_url.clone()),
        asset_size: picked.as_ref().map(|a| a.size),
        published_at: json["published_at"].as_str().map(str::to_string),
        release_notes,
        checksums_url,
        assets,
    })
}

/// Extracts the lowercase SHA-256 hex digest for `asset_name` from `SHA256SUMS.txt` content.
pub fn parse_sha256_for_asset(checksums_text: &str, asset_name: &str) -> Option<String> {
    let target = asset_name.trim();
    if target.is_empty() {
        return None;
    }
    for raw_line in checksums_text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((hash_part, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let hash = hash_part.trim();
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let file_token = rest.trim_start().trim_start_matches('*').trim();
        let basename = file_token.rsplit(['/', '\\']).next().unwrap_or(file_token);
        if basename == target {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

fn find_asset(
    assets: &[ReleaseAsset],
    pred: impl Fn(&str) -> bool,
) -> Option<ReleaseAsset> {
    assets.iter().find(|a| pred(&a.name)).cloned()
}

/// Selects the best update artifact and install mode for explicit platform flags.
pub fn select_desktop_update_asset_for(
    assets: &[ReleaseAsset],
    os: &str,
    win_is_installed: bool,
    win_has_mcp_sibling: bool,
    linux_is_appimage: bool,
    mac_in_app_bundle: bool,
) -> Option<(ReleaseAsset, UpdateInstallKind)> {
    match os {
        "windows" => {
            let is_installer = |n: &str| n.starts_with("Glyph-Setup-") && n.ends_with(".exe");
            let is_standalone = |n: &str| n.ends_with("-windows-x64.exe");
            let is_zip = |n: &str| n.ends_with("-windows-x64.zip");

            if win_is_installed {
                if let Some(a) = find_asset(assets, is_installer) {
                    return Some((a, UpdateInstallKind::WindowsInstaller));
                }
            }
            if win_has_mcp_sibling {
                if let Some(a) = find_asset(assets, is_zip) {
                    return Some((a, UpdateInstallKind::WindowsZipBundle));
                }
            }
            find_asset(assets, is_standalone)
                .map(|a| (a, UpdateInstallKind::WindowsStandalone))
                .or_else(|| {
                    find_asset(assets, is_installer).map(|a| (a, UpdateInstallKind::WindowsInstaller))
                })
                .or_else(|| {
                    find_asset(assets, is_zip).map(|a| (a, UpdateInstallKind::WindowsZipBundle))
                })
        }
        "macos" => {
            let is_zip = |n: &str| n.ends_with("-macos-universal.zip");
            let is_dmg = |n: &str| n.ends_with("-macos-universal.dmg");

            if mac_in_app_bundle {
                if let Some(a) = find_asset(assets, is_zip) {
                    return Some((a, UpdateInstallKind::MacOsAppZip));
                }
            }
            find_asset(assets, is_dmg)
                .map(|a| (a, UpdateInstallKind::MacOsDmgOpen))
                .or_else(|| {
                    find_asset(assets, is_zip).map(|a| (a, UpdateInstallKind::MacOsAppZip))
                })
        }
        "linux" => {
            let is_appimage = |n: &str| n.ends_with(".AppImage");
            let is_tarball = |n: &str| n.ends_with("-linux-x64.tar.gz");

            if linux_is_appimage {
                if let Some(a) = find_asset(assets, is_appimage) {
                    return Some((a, UpdateInstallKind::LinuxAppImage));
                }
            }
            find_asset(assets, is_tarball)
                .map(|a| (a, UpdateInstallKind::LinuxTarball))
                .or_else(|| {
                    find_asset(assets, is_appimage).map(|a| (a, UpdateInstallKind::LinuxAppImage))
                })
        }
        _ => None,
    }
}

fn select_desktop_asset_from_slice(
    assets: &[ReleaseAsset],
) -> Option<(ReleaseAsset, UpdateInstallKind)> {
    let exe_path = std::env::current_exe().ok();
    let exe_dir = exe_path.as_deref().and_then(Path::parent);
    let win_is_installed = exe_dir.is_some_and(|d| d.join("Uninstall Glyph.exe").exists());
    let win_has_mcp_sibling = exe_dir.is_some_and(|d| d.join("glyph-mcp.exe").exists());
    let linux_is_appimage = std::env::var_os("APPIMAGE").is_some_and(|v| !v.is_empty());
    let mac_in_app_bundle = exe_path.as_deref().is_some_and(|p| {
        p.ancestors()
            .any(|a| a.extension().is_some_and(|e| e == "app"))
    });
    select_desktop_update_asset_for(
        assets,
        std::env::consts::OS,
        win_is_installed,
        win_has_mcp_sibling,
        linux_is_appimage,
        mac_in_app_bundle,
    )
}

/// Chooses the appropriate update binary and install mode for the current OS & packaging layout.
pub fn select_desktop_update_asset(
    release: &ReleaseInfo,
) -> Option<(ReleaseAsset, UpdateInstallKind)> {
    select_desktop_asset_from_slice(&release.assets)
}

async fn fetch_expected_sha256(
    client: &reqwest::Client,
    checksums_url: Option<&str>,
    asset_name: &str,
) -> Option<String> {
    let url = checksums_url?;
    let resp = client
        .get(url)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body = resp.text().await.ok()?;
    parse_sha256_for_asset(&body, asset_name)
}

/// Streams the selected update asset to `dest_dir`, verifies SHA-256 when available, and supports cancellation.
pub async fn download_update_asset(
    release: &ReleaseInfo,
    dest_dir: &Path,
    cancel: CancellationToken,
    on_progress: impl Fn(UpdateDownloadProgress) + Send + 'static,
) -> Result<DownloadedUpdate> {
    if cancel.is_cancelled() {
        bail!("Update download cancelled");
    }
    ensure_rustls_ring_provider();

    let (asset, install_kind) = select_desktop_update_asset(release).ok_or_else(|| {
        anyhow!(
            "No compatible update binary found in {} for {}",
            release.tag_name,
            std::env::consts::OS
        )
    })?;

    let client = reqwest::Client::builder()
        .user_agent("Glyph-Desktop")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(600))
        .build()?;

    let expected_sha =
        fetch_expected_sha256(&client, release.checksums_url.as_deref(), &asset.name).await;

    if cancel.is_cancelled() {
        bail!("Update download cancelled");
    }

    tokio::fs::create_dir_all(dest_dir)
        .await
        .with_context(|| format!("Failed to create update directory {}", dest_dir.display()))?;

    let safe_name = Path::new(&asset.name)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Invalid release asset filename: {}", asset.name))?;

    let part_path = dest_dir.join(format!("{safe_name}.part"));
    let final_path = dest_dir.join(safe_name);
    let _ = tokio::fs::remove_file(&part_path).await;

    let resp = client
        .get(&asset.download_url)
        .send()
        .await
        .with_context(|| format!("Failed to download {}", asset.download_url))?;
    if !resp.status().is_success() {
        bail!("Download server returned HTTP {}", resp.status());
    }

    let total_bytes = resp.content_length().unwrap_or(asset.size);
    let mut stream = resp.bytes_stream();
    let mut file = tokio::fs::File::create(&part_path)
        .await
        .with_context(|| format!("Failed to create {}", part_path.display()))?;

    let mut hasher = Sha256::new();
    let mut downloaded_bytes: u64 = 0;
    let started_at = Instant::now();
    let mut last_emit = Instant::now() - Duration::from_secs(1);

    loop {
        let next_chunk = tokio::select! {
            _ = cancel.cancelled() => {
                drop(file);
                let _ = tokio::fs::remove_file(&part_path).await;
                bail!("Update download cancelled");
            }
            chunk = stream.next() => chunk,
        };

        let Some(chunk_res) = next_chunk else {
            break;
        };

        let chunk = match chunk_res {
            Ok(c) => c,
            Err(e) => {
                drop(file);
                let _ = tokio::fs::remove_file(&part_path).await;
                return Err(anyhow!("Download interrupted: {e}"));
            }
        };

        if let Err(e) = file.write_all(&chunk).await {
            drop(file);
            let _ = tokio::fs::remove_file(&part_path).await;
            return Err(anyhow!("Failed writing update file: {e}"));
        }

        hasher.update(&chunk);
        downloaded_bytes = downloaded_bytes.saturating_add(chunk.len() as u64);

        if last_emit.elapsed() >= Duration::from_millis(100) {
            last_emit = Instant::now();
            on_progress(build_progress(
                downloaded_bytes,
                total_bytes,
                started_at.elapsed().as_secs_f64(),
            ));
        }
    }

    if let Err(e) = file.flush().await {
        drop(file);
        let _ = tokio::fs::remove_file(&part_path).await;
        return Err(anyhow!("Failed flushing update file: {e}"));
    }
    drop(file);

    on_progress(build_progress(
        downloaded_bytes,
        total_bytes.max(downloaded_bytes),
        started_at.elapsed().as_secs_f64(),
    ));

    let sha256_hex = bytes_to_hex(hasher.finalize().as_slice());
    let sha256_verified = match expected_sha {
        Some(ref expected) => {
            if !sha256_hex.eq_ignore_ascii_case(expected) {
                let _ = tokio::fs::remove_file(&part_path).await;
                bail!(
                    "SHA-256 verification failed for {}: expected {}, got {}",
                    safe_name,
                    expected,
                    sha256_hex
                );
            }
            true
        }
        None => false,
    };

    if final_path.exists() {
        let _ = tokio::fs::remove_file(&final_path).await;
    }
    tokio::fs::rename(&part_path, &final_path)
        .await
        .with_context(|| format!("Failed finalizing update at {}", final_path.display()))?;

    Ok(DownloadedUpdate {
        version: release.version.clone(),
        tag_name: release.tag_name.clone(),
        asset_name: safe_name.to_string(),
        file_path: final_path,
        sha256_hex,
        sha256_verified,
        install_kind,
    })
}

fn build_progress(downloaded_bytes: u64, total_bytes: u64, elapsed_secs: f64) -> UpdateDownloadProgress {
    let percent = downloaded_bytes
        .saturating_mul(100)
        .checked_div(total_bytes)
        .map_or(0, |v| v.min(100) as u8);
    let bytes_per_sec = if elapsed_secs > 0.05 {
        downloaded_bytes as f64 / elapsed_secs
    } else {
        0.0
    };
    UpdateDownloadProgress {
        downloaded_bytes,
        total_bytes,
        percent,
        bytes_per_sec,
    }
}

/// Launches the platform-specific detached installer/replacer for a downloaded update.
pub fn launch_update_installer(downloaded: &DownloadedUpdate) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        launch_windows_updater(downloaded)
    }
    #[cfg(target_os = "linux")]
    {
        launch_linux_updater(downloaded)
    }
    #[cfg(target_os = "macos")]
    {
        launch_macos_updater(downloaded)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        let _ = downloaded;
        bail!("In-app self-update is not supported on this operating system")
    }
}

#[cfg(target_os = "windows")]
fn launch_windows_updater(downloaded: &DownloadedUpdate) -> Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW_DETACHED: u32 = 0x0800_0008;

    let mode = match downloaded.install_kind {
        UpdateInstallKind::WindowsInstaller => "installer",
        UpdateInstallKind::WindowsStandalone => "standalone",
        UpdateInstallKind::WindowsZipBundle => "zip",
        _ => bail!("Unsupported update package type for Windows"),
    };

    let current_exe = std::env::current_exe().context("Cannot locate current executable")?;
    let script_path = std::env::temp_dir().join("glyph-apply-update.ps1");
    let script_body = r#"param(
  [int]$OldPid,
  [string]$SourcePath,
  [string]$TargetExe,
  [string]$Mode
)
$ErrorActionPreference = 'Stop'
for ($i = 0; $i -lt 60; $i++) {
  if (-not (Get-Process -Id $OldPid -ErrorAction SilentlyContinue)) { break }
  Start-Sleep -Milliseconds 250
}
Start-Sleep -Milliseconds 300
if ($Mode -eq 'installer') {
  Start-Process -FilePath $SourcePath -ArgumentList @('/S', '--updated', '--force-run')
  exit 0
}
function Copy-WithRetry([string]$Src, [string]$Dst) {
  $bak = "$Dst.bak"
  if (Test-Path -LiteralPath $Dst) {
    for ($i = 0; $i -lt 20; $i++) {
      try {
        Move-Item -LiteralPath $Dst -Destination $bak -Force
        break
      } catch {
        Start-Sleep -Milliseconds 250
      }
    }
  }
  try {
    Copy-Item -LiteralPath $Src -Destination $Dst -Force
    Remove-Item -LiteralPath $bak -Force -ErrorAction SilentlyContinue
  } catch {
    if ((Test-Path -LiteralPath $bak) -and -not (Test-Path -LiteralPath $Dst)) {
      Move-Item -LiteralPath $bak -Destination $Dst -Force -ErrorAction SilentlyContinue
    }
    throw
  }
}
if ($Mode -eq 'standalone') {
  Copy-WithRetry -Src $SourcePath -Dst $TargetExe
  Remove-Item -LiteralPath $SourcePath -Force -ErrorAction SilentlyContinue
  Start-Process -FilePath $TargetExe
  exit 0
}
if ($Mode -eq 'zip') {
  $stage = Join-Path ([System.IO.Path]::GetTempPath()) ("glyph-upd-" + [Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Force -Path $stage | Out-Null
  try {
    Expand-Archive -LiteralPath $SourcePath -DestinationPath $stage -Force
    $newExe = Get-ChildItem -LiteralPath $stage -Recurse -Filter 'Glyph.exe' | Select-Object -First 1
    if (-not $newExe) {
      $newExe = Get-ChildItem -LiteralPath $stage -Recurse -Filter 'glyph-desktop.exe' | Select-Object -First 1
    }
    if (-not $newExe) { throw 'Glyph executable missing in update zip' }
    Copy-WithRetry -Src $newExe.FullName -Dst $TargetExe
    $targetDir = Split-Path -Parent $TargetExe
    $targetMcp = Join-Path $targetDir 'glyph-mcp.exe'
    $newMcp = Get-ChildItem -LiteralPath $stage -Recurse -Filter 'glyph-mcp.exe' | Select-Object -First 1
    if ($newMcp -and (Test-Path -LiteralPath $targetMcp)) {
      $oldMcp = "$targetMcp.old"
      Remove-Item -LiteralPath $oldMcp -Force -ErrorAction SilentlyContinue
      try { Move-Item -LiteralPath $targetMcp -Destination $oldMcp -Force } catch {}
      Copy-Item -LiteralPath $newMcp.FullName -Destination $targetMcp -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $SourcePath -Force -ErrorAction SilentlyContinue
    Start-Process -FilePath $TargetExe
  } finally {
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
  }
}
"#;
    std::fs::write(&script_path, script_body)
        .with_context(|| format!("Failed writing updater helper {}", script_path.display()))?;

    std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-File"])
        .arg(&script_path)
        .arg("-OldPid")
        .arg(std::process::id().to_string())
        .arg("-SourcePath")
        .arg(&downloaded.file_path)
        .arg("-TargetExe")
        .arg(&current_exe)
        .arg("-Mode")
        .arg(mode)
        .creation_flags(CREATE_NO_WINDOW_DETACHED)
        .spawn()
        .context("Failed launching Windows update helper")?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_linux_updater(downloaded: &DownloadedUpdate) -> Result<()> {
    let pid = std::process::id().to_string();
    match downloaded.install_kind {
        UpdateInstallKind::LinuxAppImage => {
            let target = std::env::var_os("APPIMAGE")
                .map(PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(
                    std::env::current_exe().context("Cannot locate current executable")?,
                );
            let script = r#"
PID="$1"; SRC="$2"; TARGET="$3"
i=0
while [ "$i" -lt 60 ] && kill -0 "$PID" 2>/dev/null; do
  sleep 0.25
  i=$((i + 1))
done
cp -f "$SRC" "$TARGET.new" && chmod +x "$TARGET.new" && mv -f "$TARGET.new" "$TARGET"
rm -f "$SRC"
nohup "$TARGET" >/dev/null 2>&1 &
"#;
            std::process::Command::new("/bin/sh")
                .arg("-c")
                .arg(script)
                .arg("glyph-updater")
                .arg(&pid)
                .arg(&downloaded.file_path)
                .arg(&target)
                .spawn()
                .context("Failed spawning Linux AppImage updater")?;
        }
        UpdateInstallKind::LinuxTarball => {
            let target = std::env::current_exe().context("Cannot locate current executable")?;
            let script = r#"
PID="$1"; SRC="$2"; TARGET="$3"
i=0
while [ "$i" -lt 60 ] && kill -0 "$PID" 2>/dev/null; do
  sleep 0.25
  i=$((i + 1))
done
STAGE="$(mktemp -d)"
tar -xzf "$SRC" -C "$STAGE"
NEW_BIN="$(find "$STAGE" -name 'glyph-desktop' -type f | head -n 1)"
if [ -n "$NEW_BIN" ]; then
  cp -f "$NEW_BIN" "$TARGET.new" && chmod +x "$TARGET.new" && mv -f "$TARGET.new" "$TARGET"
fi
TARGET_DIR="$(dirname "$TARGET")"
NEW_MCP="$(find "$STAGE" -name 'glyph-mcp' -type f | head -n 1)"
if [ -n "$NEW_MCP" ] && [ -e "$TARGET_DIR/glyph-mcp" ]; then
  cp -f "$NEW_MCP" "$TARGET_DIR/glyph-mcp.new" && chmod +x "$TARGET_DIR/glyph-mcp.new" && mv -f "$TARGET_DIR/glyph-mcp.new" "$TARGET_DIR/glyph-mcp"
fi
rm -rf "$STAGE" "$SRC"
nohup "$TARGET" >/dev/null 2>&1 &
"#;
            std::process::Command::new("/bin/sh")
                .arg("-c")
                .arg(script)
                .arg("glyph-updater")
                .arg(&pid)
                .arg(&downloaded.file_path)
                .arg(&target)
                .spawn()
                .context("Failed spawning Linux tarball updater")?;
        }
        _ => bail!("Unsupported update package type for Linux"),
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn find_macos_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .map(Path::to_path_buf)
}

#[cfg(target_os = "macos")]
fn is_dir_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".glyph-write-probe-{}", std::process::id()));
    match std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

#[cfg(target_os = "macos")]
fn launch_macos_updater(downloaded: &DownloadedUpdate) -> Result<()> {
    match downloaded.install_kind {
        UpdateInstallKind::MacOsAppZip => {
            if let Some(app_bundle) = find_macos_app_bundle() {
                let parent_ok = app_bundle.parent().is_some_and(is_dir_writable);
                if parent_ok {
                    let pid = std::process::id().to_string();
                    let script = r#"
PID="$1"; SRC="$2"; APP="$3"
i=0
while [ "$i" -lt 60 ] && kill -0 "$PID" 2>/dev/null; do
  sleep 0.25
  i=$((i + 1))
done
STAGE="$(mktemp -d)"
ditto -xk "$SRC" "$STAGE"
NEW_APP="$(find "$STAGE" -maxdepth 2 -name '*.app' -type d | head -n 1)"
if [ -n "$NEW_APP" ]; then
  BAK="$APP.bak"
  rm -rf "$BAK"
  mv "$APP" "$BAK"
  if mv "$NEW_APP" "$APP"; then
    rm -rf "$BAK"
    xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true
  else
    mv "$BAK" "$APP" 2>/dev/null || true
  fi
fi
rm -rf "$STAGE" "$SRC"
open -n "$APP"
"#;
                    std::process::Command::new("/bin/sh")
                        .arg("-c")
                        .arg(script)
                        .arg("glyph-updater")
                        .arg(&pid)
                        .arg(&downloaded.file_path)
                        .arg(&app_bundle)
                        .spawn()
                        .context("Failed spawning macOS app bundle updater")?;
                    return Ok(());
                }
            }
            std::process::Command::new("open")
                .arg("-R")
                .arg(&downloaded.file_path)
                .spawn()
                .context("Failed revealing downloaded macOS update archive")?;
        }
        UpdateInstallKind::MacOsDmgOpen => {
            std::process::Command::new("open")
                .arg(&downloaded.file_path)
                .spawn()
                .context("Failed opening macOS disk image")?;
        }
        _ => bail!("Unsupported update package type for macOS"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn sample_release_assets() -> Vec<ReleaseAsset> {
        vec![
            ReleaseAsset {
                name: "Glyph-Setup-3.0.0.exe".into(),
                download_url: "https://example.invalid/Glyph-Setup-3.0.0.exe".into(),
                size: 1000,
            },
            ReleaseAsset {
                name: "Glyph-v3.0.0-windows-x64.exe".into(),
                download_url: "https://example.invalid/Glyph-v3.0.0-windows-x64.exe".into(),
                size: 900,
            },
            ReleaseAsset {
                name: "Glyph-v3.0.0-windows-x64.zip".into(),
                download_url: "https://example.invalid/Glyph-v3.0.0-windows-x64.zip".into(),
                size: 800,
            },
            ReleaseAsset {
                name: "Glyph-v3.0.0-macos-universal.dmg".into(),
                download_url: "https://example.invalid/Glyph-v3.0.0-macos-universal.dmg".into(),
                size: 1100,
            },
            ReleaseAsset {
                name: "Glyph-v3.0.0-macos-universal.zip".into(),
                download_url: "https://example.invalid/Glyph-v3.0.0-macos-universal.zip".into(),
                size: 1050,
            },
            ReleaseAsset {
                name: "Glyph-v3.0.0-linux-x64.AppImage".into(),
                download_url: "https://example.invalid/Glyph-v3.0.0-linux-x64.AppImage".into(),
                size: 950,
            },
            ReleaseAsset {
                name: "Glyph-v3.0.0-linux-x64.tar.gz".into(),
                download_url: "https://example.invalid/Glyph-v3.0.0-linux-x64.tar.gz".into(),
                size: 920,
            },
        ]
    }

    #[test]
    fn semver_comparison_handles_prerelease_and_v_prefix() {
        assert!(is_newer_version("3.0.0-alpha.1", "v3.0.0"));
        assert!(is_newer_version("3.0.0-alpha.1", "v3.0.0-alpha.2"));
        assert!(is_newer_version("v2.7.6", "3.0.0"));
        assert!(!is_newer_version("3.0.0", "v3.0.0-alpha.9"));
        assert!(!is_newer_version("v2.7.6", "2.7.6"));
    }

    #[test]
    fn parses_sha256_entries_with_spaces_or_binary_star() {
        let h1 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let h2 = "FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210";
        let txt = format!(
            "# comment\n{h1}  Glyph-Setup-3.0.0.exe\n{h2} *dist/Glyph-v3.0.0-windows-x64.zip\n"
        );
        assert_eq!(
            parse_sha256_for_asset(&txt, "Glyph-Setup-3.0.0.exe").as_deref(),
            Some(h1)
        );
        assert_eq!(
            parse_sha256_for_asset(&txt, "Glyph-v3.0.0-windows-x64.zip").as_deref(),
            Some(&h2.to_ascii_lowercase()[..])
        );
        assert_eq!(parse_sha256_for_asset(&txt, "missing.exe"), None);
    }

    #[test]
    fn asset_selection_covers_all_platforms_and_modes() {
        let assets = sample_release_assets();
        assert_eq!(
            select_desktop_update_asset_for(&assets, "windows", true, true, false, false)
                .map(|(a, k)| (a.name, k)),
            Some((
                "Glyph-Setup-3.0.0.exe".into(),
                UpdateInstallKind::WindowsInstaller
            ))
        );
        assert_eq!(
            select_desktop_update_asset_for(&assets, "windows", false, true, false, false)
                .map(|(a, k)| (a.name, k)),
            Some((
                "Glyph-v3.0.0-windows-x64.zip".into(),
                UpdateInstallKind::WindowsZipBundle
            ))
        );
        assert_eq!(
            select_desktop_update_asset_for(&assets, "windows", false, false, false, false)
                .map(|(a, k)| (a.name, k)),
            Some((
                "Glyph-v3.0.0-windows-x64.exe".into(),
                UpdateInstallKind::WindowsStandalone
            ))
        );
        assert_eq!(
            select_desktop_update_asset_for(&assets, "macos", false, false, false, true)
                .map(|(a, k)| (a.name, k)),
            Some((
                "Glyph-v3.0.0-macos-universal.zip".into(),
                UpdateInstallKind::MacOsAppZip
            ))
        );
        assert_eq!(
            select_desktop_update_asset_for(&assets, "macos", false, false, false, false)
                .map(|(a, k)| (a.name, k)),
            Some((
                "Glyph-v3.0.0-macos-universal.dmg".into(),
                UpdateInstallKind::MacOsDmgOpen
            ))
        );
        assert_eq!(
            select_desktop_update_asset_for(&assets, "linux", false, false, true, false)
                .map(|(a, k)| (a.name, k)),
            Some((
                "Glyph-v3.0.0-linux-x64.AppImage".into(),
                UpdateInstallKind::LinuxAppImage
            ))
        );
        assert_eq!(
            select_desktop_update_asset_for(&assets, "linux", false, false, false, false)
                .map(|(a, k)| (a.name, k)),
            Some((
                "Glyph-v3.0.0-linux-x64.tar.gz".into(),
                UpdateInstallKind::LinuxTarball
            ))
        );
    }

    async fn spawn_mock_release_server(
        payload: Vec<u8>,
        checksums_override: Option<String>,
        slow_chunks: bool,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}");
        let sha_hex = checksums_override.unwrap_or_else(|| {
            let d = bytes_to_hex(Sha256::digest(&payload).as_slice());
            [
                format!("{d}  Glyph-Setup-3.0.0.exe"),
                format!("{d}  Glyph-v3.0.0-windows-x64.exe"),
                format!("{d}  Glyph-v3.0.0-windows-x64.zip"),
                format!("{d}  Glyph-v3.0.0-macos-universal.dmg"),
                format!("{d}  Glyph-v3.0.0-macos-universal.zip"),
                format!("{d}  Glyph-v3.0.0-linux-x64.AppImage"),
                format!("{d}  Glyph-v3.0.0-linux-x64.tar.gz"),
            ]
            .join("\n")
        });
        let base_copy = base_url.clone();
        tokio::spawn(async move {
            for _ in 0..8 {
                let Ok((mut sock, _)) = listener.accept().await else {
                    break;
                };
                let payload_c = payload.clone();
                let sha_c = sha_hex.clone();
                let base_i = base_copy.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let path = req
                        .lines()
                        .next()
                        .and_then(|l| l.split_whitespace().nth(1))
                        .unwrap_or("/");
                    if path == "/repos/TheLunatic1/Glyph/releases/latest" {
                        let json_body = serde_json::json!({
                            "tag_name": "v3.0.0",
                            "html_url": format!("{base_i}/releases/v3.0.0"),
                            "body": "## v3.0.0\n- Pure Rust auto-updater",
                            "published_at": "2026-11-22T00:00:00Z",
                            "assets": [
                                {"name": "SHA256SUMS.txt", "browser_download_url": format!("{base_i}/dl/SHA256SUMS.txt"), "size": sha_c.len()},
                                {"name": "Glyph-Setup-3.0.0.exe", "browser_download_url": format!("{base_i}/dl/Glyph-Setup-3.0.0.exe"), "size": payload_c.len()},
                                {"name": "Glyph-v3.0.0-windows-x64.exe", "browser_download_url": format!("{base_i}/dl/Glyph-v3.0.0-windows-x64.exe"), "size": payload_c.len()},
                                {"name": "Glyph-v3.0.0-windows-x64.zip", "browser_download_url": format!("{base_i}/dl/Glyph-v3.0.0-windows-x64.zip"), "size": payload_c.len()},
                                {"name": "Glyph-v3.0.0-macos-universal.dmg", "browser_download_url": format!("{base_i}/dl/Glyph-v3.0.0-macos-universal.dmg"), "size": payload_c.len()},
                                {"name": "Glyph-v3.0.0-macos-universal.zip", "browser_download_url": format!("{base_i}/dl/Glyph-v3.0.0-macos-universal.zip"), "size": payload_c.len()},
                                {"name": "Glyph-v3.0.0-linux-x64.AppImage", "browser_download_url": format!("{base_i}/dl/Glyph-v3.0.0-linux-x64.AppImage"), "size": payload_c.len()},
                                {"name": "Glyph-v3.0.0-linux-x64.tar.gz", "browser_download_url": format!("{base_i}/dl/Glyph-v3.0.0-linux-x64.tar.gz"), "size": payload_c.len()},
                            ]
                        })
                        .to_string();
                        let hdr = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            json_body.len()
                        );
                        let _ = sock.write_all(hdr.as_bytes()).await;
                        let _ = sock.write_all(json_body.as_bytes()).await;
                    } else if path == "/dl/SHA256SUMS.txt" {
                        let hdr = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            sha_c.len()
                        );
                        let _ = sock.write_all(hdr.as_bytes()).await;
                        let _ = sock.write_all(sha_c.as_bytes()).await;
                    } else if path.starts_with("/dl/") {
                        let hdr = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            payload_c.len()
                        );
                        let _ = sock.write_all(hdr.as_bytes()).await;
                        if slow_chunks {
                            for chunk in payload_c.chunks(8192) {
                                if sock.write_all(chunk).await.is_err() {
                                    break;
                                }
                                tokio::time::sleep(Duration::from_millis(25)).await;
                            }
                        } else {
                            let _ = sock.write_all(&payload_c).await;
                        }
                    }
                });
            }
        });
        base_url
    }

    #[tokio::test]
    async fn streaming_download_verifies_sha256_rejects_mismatch_and_cleans_up_cancel() {
        let payload = vec![0x5A_u8; 65_536];
        let base_ok = spawn_mock_release_server(payload.clone(), None, false).await;
        let rel = fetch_latest_release_from_base(&base_ok, "TheLunatic1/Glyph", None)
            .await
            .unwrap();
        assert_eq!(rel.tag_name, "v3.0.0");
        assert!(rel.release_notes.contains("Pure Rust auto-updater"));
        assert!(rel.checksums_url.is_some());

        let tmp = std::env::temp_dir().join(format!("glyph-upd-test-{}", uuid::Uuid::new_v4()));
        let cancel_ok = CancellationToken::new();
        let seen_100 = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen_100_c = std::sync::Arc::clone(&seen_100);
        let dl = download_update_asset(&rel, &tmp, cancel_ok, move |p| {
            if p.percent == 100 {
                seen_100_c.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        })
        .await
        .unwrap();
        assert!(dl.sha256_verified);
        assert!(dl.file_path.exists());
        assert!(seen_100.load(std::sync::atomic::Ordering::Relaxed));

        // Mismatch rejection test
        let bad_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let bad_sums = [
            format!("{bad_hex}  Glyph-Setup-3.0.0.exe"),
            format!("{bad_hex}  Glyph-v3.0.0-windows-x64.exe"),
            format!("{bad_hex}  Glyph-v3.0.0-windows-x64.zip"),
            format!("{bad_hex}  Glyph-v3.0.0-macos-universal.dmg"),
            format!("{bad_hex}  Glyph-v3.0.0-macos-universal.zip"),
            format!("{bad_hex}  Glyph-v3.0.0-linux-x64.AppImage"),
            format!("{bad_hex}  Glyph-v3.0.0-linux-x64.tar.gz"),
        ]
        .join("\n");
        let base_bad = spawn_mock_release_server(payload.clone(), Some(bad_sums), false).await;
        let rel_bad = fetch_latest_release_from_base(&base_bad, "TheLunatic1/Glyph", None)
            .await
            .unwrap();
        let tmp_bad = tmp.join("bad");
        let err_bad = download_update_asset(&rel_bad, &tmp_bad, CancellationToken::new(), |_| {})
            .await
            .unwrap_err();
        assert!(err_bad.to_string().contains("SHA-256 verification failed"));
        assert!(!tmp_bad.join(format!("{}.part", dl.asset_name)).exists());

        // Cancel cleanup test
        let base_slow = spawn_mock_release_server(payload, None, true).await;
        let rel_slow = fetch_latest_release_from_base(&base_slow, "TheLunatic1/Glyph", None)
            .await
            .unwrap();
        let cancel_abort = CancellationToken::new();
        let cancel_trigger = cancel_abort.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(40)).await;
            cancel_trigger.cancel();
        });
        let tmp_cancel = tmp.join("cancel");
        let err_cancel = download_update_asset(&rel_slow, &tmp_cancel, cancel_abort, |_| {})
            .await
            .unwrap_err();
        assert!(err_cancel.to_string().contains("cancelled"));
        assert!(!tmp_cancel.join(format!("{}.part", dl.asset_name)).exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
