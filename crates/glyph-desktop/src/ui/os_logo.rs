use crate::theme::{
    rgba, BRAND_400, DARK_700, DARK_800, DARK_900, FONT_UI_BOLD,
};
use iced::widget::{container, image, text};
use iced::{alignment, Border, Color, Element, Length};
use resvg::tiny_skia;
use resvg::usvg;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

// Crisp bundled offline fallback SVGs (used instantaneously on first launch or when offline)
const ORACLE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#C74634"/><rect x="20" y="34" width="60" height="32" rx="16" fill="none" stroke="#ffffff" stroke-width="10"/></svg>"##;
const UBUNTU_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#E95420"/><circle cx="50" cy="21" r="7" fill="#ffffff"/><circle cx="25" cy="64" r="7" fill="#ffffff"/><circle cx="75" cy="64" r="7" fill="#ffffff"/><circle cx="50" cy="50" r="20" fill="none" stroke="#ffffff" stroke-width="8"/></svg>"##;
const DEBIAN_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#A80030"/><path d="M55 24c16 2 24 14 21 28-3 13-16 21-29 18-10-3-16-12-13-22 3-8 11-12 19-10 6 2 10 8 8 14-2 5-7 7-12 6" fill="none" stroke="#ffffff" stroke-width="7" stroke-linecap="round"/></svg>"##;
const KALI_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#1D4ED8"/><path d="M22 66c16-24 38-34 60-32-16 8-26 16-34 26 11-5 22-4 32 0-16 2-28 8-38 16-6-4-12-7-20-10z" fill="#ffffff"/></svg>"##;
const ARCH_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#1793D1"/><path d="M50 18 L77 77 Q62 67 50 72 Q38 67 23 77 Z" fill="#ffffff"/></svg>"##;
const FEDORA_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#294172"/><path d="M56 28a14 14 0 0 1 14 14v8H44v14a14 14 0 0 1-14-14" fill="none" stroke="#ffffff" stroke-width="9" stroke-linecap="round"/></svg>"##;
const CENTOS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#262577"/><rect x="28" y="28" width="20" height="20" rx="2" fill="#9CCD2A"/><rect x="52" y="28" width="20" height="20" rx="2" fill="#932279"/><rect x="28" y="52" width="20" height="20" rx="2" fill="#EFA724"/><rect x="52" y="52" width="20" height="20" rx="2" fill="#ffffff"/></svg>"##;
const ROCKY_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#10B981"/><path d="M26 70 L46 34 L58 52 L68 38 L78 70 Z" fill="#064E3B"/><path d="M28 70 L46 38 L57 55 L67 42 L76 70" fill="none" stroke="#ffffff" stroke-width="5" stroke-linejoin="round"/></svg>"##;
const ALMA_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#0F4266"/><circle cx="50" cy="24" r="8" fill="#FF4649"/><circle cx="75" cy="42" r="8" fill="#FFBE2E"/><circle cx="65" cy="72" r="8" fill="#0095D9"/><circle cx="35" cy="72" r="8" fill="#86BE3F"/><circle cx="25" cy="42" r="8" fill="#23B2EE"/></svg>"##;
const REDHAT_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#CC0000"/><ellipse cx="50" cy="57" rx="30" ry="12" fill="#111827"/><path d="M31 54c4-14 11-22 19-22s15 8 19 22z" fill="#ffffff"/><rect x="34" y="48" width="32" height="5" rx="2" fill="#CC0000"/></svg>"##;
const ALPINE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#0D597F"/><polygon points="40,26 62,70 18,70" fill="#ffffff"/><polygon points="62,36 82,70 42,70" fill="#93C5FD"/></svg>"##;
const NIXOS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#5277C3"/><polygon points="50,22 74,36 74,64 50,78 26,64 26,36" fill="none" stroke="#ffffff" stroke-width="7"/></svg>"##;
const OPENSUSE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#73BA25"/><circle cx="50" cy="50" r="21" fill="none" stroke="#ffffff" stroke-width="8"/></svg>"##;
const RASPBERRY_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#C51A4A"/><ellipse cx="50" cy="56" rx="20" ry="22" fill="#ffffff"/><path d="M36 28c6 4 12 4 14 8 2-4 8-4 14-8" fill="none" stroke="#4ADE80" stroke-width="6" stroke-linecap="round"/></svg>"##;
const AWS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#232F3E"/><path d="M26 58c16 12 34 12 48 0" fill="none" stroke="#FF9900" stroke-width="7" stroke-linecap="round"/><path d="M68 54l8 3-2 8" fill="none" stroke="#FF9900" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const APPLE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#27272A"/><path d="M62 49c0-8 7-12 7-12-4-5-10-6-12-6-5 0-9 3-11 3-3 0-6-3-10-3-6 0-11 4-14 9-5 10-2 25 4 33 3 4 6 8 10 8 4 0 6-2 10-2s6 2 10 2c4 0 7-4 10-8 3-4 4-9 4-9s-9-3-9-15z" fill="#E5E7EB"/><path d="M53 26c2-3 5-5 8-5-1 4-3 7-6 9-3 2-6 3-8 2 1-3 3-4 6-6z" fill="#E5E7EB"/></svg>"##;
const FREEBSD_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#AB2B28"/><circle cx="50" cy="54" r="19" fill="#ffffff"/><polygon points="27,34 38,24 41,40" fill="#ffffff"/><polygon points="73,34 62,24 59,40" fill="#ffffff"/></svg>"##;
const LINUX_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#334155"/><circle cx="50" cy="36" r="14" fill="#facc15"/><ellipse cx="50" cy="64" rx="18" ry="18" fill="#f8fafc"/></svg>"##;

static ICON_CACHE_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static FETCHED_SVGS: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();
static ATTEMPTED_SLUGS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static RASTER_CACHE: OnceLock<Mutex<HashMap<(String, u32), image::Handle>>> = OnceLock::new();
static RASTER_FAILED_SVGS: OnceLock<Mutex<HashSet<(String, u32)>>> = OnceLock::new();

fn sanitize_slug(slug: &str) -> Option<String> {
    let clean = slug.trim().to_ascii_lowercase();
    if clean.is_empty()
        || !clean
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(clean)
}

pub fn devicon_slug(os: &str) -> Option<String> {
    let key = os.trim().to_ascii_lowercase();
    if key.is_empty() {
        return None;
    }
    match key.as_str() {
        // Devicons' oracle icon is a tiny 8:1 wordmark; use the bold circular Oracle O-emblem instead.
        "ol" | "oracle" | "oraclelinux" | "oracle-linux" => None,
        "ubuntu" | "pop" | "pop!_os" => Some("ubuntu".to_string()),
        "debian" | "linuxmint" | "mint" => Some("debian".to_string()),
        "kali" | "kali-linux" | "kalilinux" => Some("kalilinux".to_string()),
        "raspbian" | "raspberrypi" => Some("raspberrypi".to_string()),
        "rocky" | "rockylinux" | "rocky-linux" => Some("rockylinux".to_string()),
        "almalinux" | "alma" => Some("almalinux".to_string()),
        "rhel" | "redhat" | "red hat" | "red-hat" => Some("redhat".to_string()),
        "centos" => Some("centos".to_string()),
        "fedora" => Some("fedora".to_string()),
        "arch" | "archlinux" | "manjaro" | "endeavouros" | "garuda" => {
            Some("archlinux".to_string())
        }
        "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" | "suse" => {
            Some("opensuse".to_string())
        }
        "nixos" => Some("nixos".to_string()),
        "gentoo" => Some("gentoo".to_string()),
        "amazon" | "amzn" => Some("amazonwebservices".to_string()),
        "darwin" | "macos" => Some("apple".to_string()),
        "linux" | "void" => Some("linux".to_string()),
        "alpine" | "freebsd" | "openbsd" | "netbsd" | "openwrt" => None,
        _ => sanitize_slug(&key),
    }
}

pub fn os_display_name(os: &str) -> String {
    let raw = os.trim();
    match raw.to_ascii_lowercase().as_str() {
        "ol" | "oracle" | "oraclelinux" | "oracle-linux" => "Oracle".to_string(),
        "rhel" | "redhat" | "red hat" | "red-hat" => "RHEL".to_string(),
        "rocky" | "rockylinux" | "rocky-linux" => "Rocky".to_string(),
        "almalinux" | "alma" => "Alma".to_string(),
        "ubuntu" => "Ubuntu".to_string(),
        "debian" => "Debian".to_string(),
        "kali" | "kali-linux" | "kalilinux" => "Kali".to_string(),
        "raspbian" => "Raspbian".to_string(),
        "pop" | "pop!_os" => "Pop!_OS".to_string(),
        "linuxmint" | "mint" => "Mint".to_string(),
        "centos" => "CentOS".to_string(),
        "fedora" => "Fedora".to_string(),
        "arch" | "archlinux" => "Arch".to_string(),
        "manjaro" => "Manjaro".to_string(),
        "alpine" => "Alpine".to_string(),
        "nixos" => "NixOS".to_string(),
        "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" | "suse" => "openSUSE".to_string(),
        "amzn" | "amazon" => "Amazon".to_string(),
        "darwin" | "macos" => "macOS".to_string(),
        "freebsd" => "FreeBSD".to_string(),
        "openwrt" => "OpenWrt".to_string(),
        "gentoo" => "Gentoo".to_string(),
        "void" => "Void".to_string(),
        "linux" => "Linux".to_string(),
        _ => {
            let mut chars = raw.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

pub fn set_icon_cache_dir(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    let cell = ICON_CACHE_DIR.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = cell.lock() {
        *guard = Some(dir);
    }
}

fn disk_cache_path(slug: &str) -> Option<PathBuf> {
    let clean = sanitize_slug(slug)?;
    let cell = ICON_CACHE_DIR.get()?;
    let guard = cell.lock().ok()?;
    guard.as_ref().map(|d| d.join(format!("{clean}.svg")))
}

pub fn load_cached_svg_from_disk(slug: &str) -> Option<Vec<u8>> {
    let path = disk_cache_path(slug)?;
    let bytes = fs::read(path).ok()?;
    if bytes.is_empty() || bytes.len() > 512 * 1024 {
        return None;
    }
    usvg::Tree::from_data(&bytes, &usvg::Options::default()).ok()?;
    Some(bytes)
}

fn invalidate_raster_entries(slug: &str) {
    let prefix = format!("fetched:{slug}");
    if let Some(cache) = RASTER_CACHE.get() {
        if let Ok(mut guard) = cache.lock() {
            guard.retain(|(k, _), _| k != &prefix);
        }
    }
    if let Some(failed) = RASTER_FAILED_SVGS.get() {
        if let Ok(mut guard) = failed.lock() {
            guard.retain(|(k, _)| k != &prefix);
        }
    }
}

pub fn register_fetched_svg(slug: &str, bytes: Vec<u8>) -> bool {
    let Some(clean) = sanitize_slug(slug) else {
        return false;
    };
    if bytes.is_empty() || bytes.len() > 512 * 1024 {
        return false;
    }
    let normalized = normalize_fetched_svg(&clean, bytes);
    if usvg::Tree::from_data(&normalized, &usvg::Options::default()).is_err() {
        return false;
    }

    if let Some(path) = disk_cache_path(&clean) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let tmp = path.with_extension("svg.tmp");
        if fs::write(&tmp, &normalized).is_ok() {
            let _ = fs::rename(&tmp, &path);
        }
    }

    let store = FETCHED_SVGS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = store.lock() {
        guard.insert(clean.clone(), normalized);
    }
    invalidate_raster_entries(&clean);
    true
}

fn normalize_fetched_svg(slug: &str, bytes: Vec<u8>) -> Vec<u8> {
    if slug == "apple" {
        if let Ok(text) = std::str::from_utf8(&bytes) {
            if !text.contains("fill=") {
                return text.replacen("<svg", r##"<svg fill="#E5E7EB""##, 1).into_bytes();
            }
        }
    }
    bytes
}

/// Returns a Devicons slug if the OS icon isn't already in memory/disk cache and hasn't been attempted yet.
pub fn needs_fetch(os: &str) -> Option<String> {
    let slug = devicon_slug(os)?;
    {
        let store = FETCHED_SVGS.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(guard) = store.lock() {
            if guard.contains_key(&slug) {
                return None;
            }
        }
    }
    if let Some(disk_bytes) = load_cached_svg_from_disk(&slug) {
        let store = FETCHED_SVGS.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(mut guard) = store.lock() {
            guard.insert(slug, disk_bytes);
        }
        return None;
    }

    let attempted = ATTEMPTED_SLUGS.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut guard) = attempted.lock() {
        if !guard.insert(slug.clone()) {
            return None;
        }
    }
    Some(slug)
}

pub async fn fetch_devicon_svg(slug: &str) -> Option<Vec<u8>> {
    let clean = sanitize_slug(slug)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("Glyph-Desktop/3.0")
        .build()
        .ok()?;

    let urls = [
        format!("https://cdn.jsdelivr.net/gh/devicons/devicon/icons/{clean}/{clean}-original.svg"),
        format!("https://cdn.jsdelivr.net/gh/devicons/devicon/icons/{clean}/{clean}-plain.svg"),
    ];

    for url in urls {
        let Ok(resp) = client.get(&url).send().await else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(bytes) = resp.bytes().await else {
            continue;
        };
        if bytes.is_empty() || bytes.len() > 512 * 1024 {
            continue;
        }
        let vec = bytes.to_vec();
        if usvg::Tree::from_data(&vec, &usvg::Options::default()).is_ok() {
            return Some(vec);
        }
    }
    None
}

pub(crate) fn rasterize_svg_raw(svg_bytes: &[u8], target_px: u32) -> Option<Vec<u8>> {
    let target_px = target_px.clamp(16, 512);
    let tree = usvg::Tree::from_data(svg_bytes, &usvg::Options::default()).ok()?;
    let tree_size = tree.size();
    let tw = tree_size.width();
    let th = tree_size.height();
    if tw <= 0.0 || th <= 0.0 {
        return None;
    }

    let mut pixmap = tiny_skia::Pixmap::new(target_px, target_px)?;
    let inner = (target_px as f32) * 0.86;
    let scale = (inner / tw).min(inner / th);
    let tx = (target_px as f32 - tw * scale) * 0.5;
    let ty = (target_px as f32 - th * scale) * 0.5;
    let transform = tiny_skia::Transform::from_scale(scale, scale).post_translate(tx, ty);

    let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        resvg::render(&tree, transform, &mut pixmap.as_mut());
    }))
    .is_ok();
    if !ok {
        return None;
    }

    // `tiny_skia::Pixmap` uses premultiplied RGBA, whereas `iced_wgpu`'s image shader blends
    // with `SrcAlpha / OneMinusSrcAlpha`. Un-premultiply to avoid dark halos and thin edges.
    let mut rgba = pixmap.take();
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = px[3] as u32;
        if a > 0 && a < 255 {
            px[0] = ((px[0] as u32 * 255 + a / 2) / a).min(255) as u8;
            px[1] = ((px[1] as u32 * 255 + a / 2) / a).min(255) as u8;
            px[2] = ((px[2] as u32 * 255 + a / 2) / a).min(255) as u8;
        }
    }
    Some(rgba)
}

pub fn rasterize_svg_rgba(svg_bytes: &[u8], target_px: u32) -> Option<image::Handle> {
    let target_px = target_px.clamp(16, 512);
    let rgba = rasterize_svg_raw(svg_bytes, target_px)?;
    Some(image::Handle::from_rgba(target_px, target_px, rgba))
}

fn bundled_svg(os_lower: &str) -> Option<(&'static str, &'static str)> {
    match os_lower {
        "ol" | "oracle" | "oraclelinux" | "oracle-linux" => Some(("oracle", ORACLE_SVG)),
        "ubuntu" | "pop" | "pop!_os" => Some(("ubuntu", UBUNTU_SVG)),
        "debian" | "linuxmint" | "mint" => Some(("debian", DEBIAN_SVG)),
        "kali" | "kali-linux" | "kalilinux" => Some(("kali", KALI_SVG)),
        "raspbian" | "raspberrypi" => Some(("raspberrypi", RASPBERRY_SVG)),
        "arch" | "archlinux" | "manjaro" | "endeavouros" | "garuda" => Some(("arch", ARCH_SVG)),
        "fedora" => Some(("fedora", FEDORA_SVG)),
        "centos" => Some(("centos", CENTOS_SVG)),
        "rocky" | "rockylinux" | "rocky-linux" => Some(("rocky", ROCKY_SVG)),
        "almalinux" | "alma" => Some(("alma", ALMA_SVG)),
        "rhel" | "redhat" | "red hat" | "red-hat" => Some(("redhat", REDHAT_SVG)),
        "alpine" => Some(("alpine", ALPINE_SVG)),
        "nixos" => Some(("nixos", NIXOS_SVG)),
        "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" | "suse" => {
            Some(("opensuse", OPENSUSE_SVG))
        }
        "amazon" | "amzn" | "amazonwebservices" => Some(("aws", AWS_SVG)),
        "darwin" | "macos" => Some(("apple", APPLE_SVG)),
        "freebsd" | "openbsd" | "netbsd" => Some(("freebsd", FREEBSD_SVG)),
        "linux" | "gentoo" | "void" => Some(("linux", LINUX_SVG)),
        _ => None,
    }
}

fn cached_raster_handle(cache_key: &str, svg_bytes: &[u8], target_px: u32) -> Option<image::Handle> {
    let key = (cache_key.to_string(), target_px);
    {
        let failed = RASTER_FAILED_SVGS.get_or_init(|| Mutex::new(HashSet::new()));
        if let Ok(guard) = failed.lock() {
            if guard.contains(&key) {
                return None;
            }
        }
    }
    let cache = RASTER_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(h) = guard.get(&key) {
            return Some(h.clone());
        }
    }
    if let Some(handle) = rasterize_svg_rgba(svg_bytes, target_px) {
        if let Ok(mut guard) = cache.lock() {
            guard.insert(key, handle.clone());
        }
        Some(handle)
    } else {
        let failed = RASTER_FAILED_SVGS.get_or_init(|| Mutex::new(HashSet::new()));
        if let Ok(mut guard) = failed.lock() {
            guard.insert(key);
        }
        None
    }
}

pub fn os_logo<'a, Message: 'a>(
    os: Option<&str>,
    server_name: &str,
    size: f32,
) -> Element<'a, Message> {
    let snapped = size.round().max(16.0);
    let target_px = ((snapped as u32) * 2).clamp(32, 192);

    if let Some(os_raw) = os {
        let key = os_raw.trim().to_ascii_lowercase();
        let slug_opt = devicon_slug(&key);

        if let Some(slug) = slug_opt.as_deref() {
            let fetched_bytes = FETCHED_SVGS
                .get()
                .and_then(|m| m.lock().ok())
                .and_then(|g| g.get(slug).cloned())
                .or_else(|| {
                    load_cached_svg_from_disk(slug).inspect(|bytes| {
                        let store = FETCHED_SVGS.get_or_init(|| Mutex::new(HashMap::new()));
                        if let Ok(mut g) = store.lock() {
                            g.insert(slug.to_string(), bytes.clone());
                        }
                    })
                });

            if let Some(svg_bytes) = fetched_bytes {
                let cache_id = format!("fetched:{slug}");
                if let Some(handle) = cached_raster_handle(&cache_id, &svg_bytes, target_px) {
                    return framed_logo(handle, snapped);
                }
            }
        }

        if let Some((tag, raw_svg)) = bundled_svg(&key) {
            let cache_id = format!("bundled:{tag}");
            if let Some(handle) = cached_raster_handle(&cache_id, raw_svg.as_bytes(), target_px) {
                return image(handle)
                    .width(Length::Fixed(snapped))
                    .height(Length::Fixed(snapped))
                    .filter_method(image::FilterMethod::Linear)
                    .into();
            }
        }

        return initials_badge(os_raw, snapped);
    }

    initials_badge(server_name, snapped)
}

fn framed_logo<'a, Message: 'a>(handle: image::Handle, size: f32) -> Element<'a, Message> {
    let pad = (size * 0.12).round().max(2.0) as u16;
    let img_sz = (size - (pad as f32) * 2.0).max(12.0);
    container(
        image(handle)
            .width(Length::Fixed(img_sz))
            .height(Length::Fixed(img_sz))
            .filter_method(image::FilterMethod::Linear),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .padding(pad)
    .center_x(Length::Fixed(size))
    .center_y(Length::Fixed(size))
    .style(move |_| container::Style {
        background: Some(iced::Background::Color(DARK_900)),
        border: Border {
            color: rgba(Color::WHITE, 0.08),
            width: 1.0,
            radius: (size * 0.26).into(),
        },
        snap: true,
        ..Default::default()
    })
    .into()
}

fn initials_badge<'a, Message: 'a>(label: &str, size: f32) -> Element<'a, Message> {
    let initials: String = label
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let txt = if initials.is_empty() {
        "GH".to_string()
    } else {
        initials
    };

    container(
        text(txt)
            .size((size * 0.36).max(11.0))
            .font(FONT_UI_BOLD)
            .color(BRAND_400)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .center_x(Length::Fixed(size))
    .center_y(Length::Fixed(size))
    .style(move |_| container::Style {
        background: Some(iced::Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 999.0.into(),
        },
        snap: true,
        ..Default::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_oracle_and_common_distros() {
        assert_eq!(devicon_slug("ol"), None);
        assert_eq!(bundled_svg("ol").map(|(t, _)| t), Some("oracle"));
        assert_eq!(os_display_name("ol"), "Oracle");
        assert_eq!(devicon_slug("ubuntu").as_deref(), Some("ubuntu"));
        assert_eq!(os_display_name("ubuntu"), "Ubuntu");
        assert_eq!(devicon_slug("rocky").as_deref(), Some("rockylinux"));
        assert_eq!(devicon_slug("kali").as_deref(), Some("kalilinux"));
        assert_eq!(devicon_slug("alpine"), None);
        assert_eq!(devicon_slug("freebsd"), None);
    }

    #[test]
    fn rasterizes_bundled_svgs_with_straight_alpha() {
        for raw in [ORACLE_SVG, UBUNTU_SVG, ALPINE_SVG] {
            let rgba = rasterize_svg_raw(raw.as_bytes(), 80).expect("svg should rasterize");
            assert_eq!(rgba.len(), 80 * 80 * 4);
            let mut non_zero_alpha = 0usize;
            for px in rgba.as_chunks::<4>().0 {
                if px[3] > 0 {
                    non_zero_alpha += 1;
                }
            }
            assert!(non_zero_alpha > 100);
        }
    }
}
