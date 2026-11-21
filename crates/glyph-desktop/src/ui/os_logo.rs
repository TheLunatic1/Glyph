use crate::theme::{BRAND_400, DARK_700};
use iced::widget::{container, svg, text};
use iced::{alignment, Border, Color, Element, Length};

const UBUNTU_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#E95420"/><circle cx="50" cy="21" r="7" fill="#fff"/><circle cx="25" cy="64" r="7" fill="#fff"/><circle cx="75" cy="64" r="7" fill="#fff"/><circle cx="50" cy="50" r="20" fill="none" stroke="#fff" stroke-width="8"/></svg>"##;
const DEBIAN_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#A80030"/><path d="M55 24c16 2 24 14 21 28-3 13-16 21-29 18-10-3-16-12-13-22 3-8 11-12 19-10 6 2 10 8 8 14-2 5-7 7-12 6" fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round"/></svg>"##;
const ARCH_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#1793D1"/><path d="M50 18 L77 77 Q62 67 50 72 Q38 67 23 77 Z" fill="#fff"/></svg>"##;
const FEDORA_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#294172"/><path d="M56 28a14 14 0 0 1 14 14v8H44v14a14 14 0 0 1-14-14" fill="none" stroke="#fff" stroke-width="9" stroke-linecap="round"/></svg>"##;
const CENTOS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#262577"/><rect x="28" y="28" width="20" height="20" rx="2" fill="#9CCD2A"/><rect x="52" y="28" width="20" height="20" rx="2" fill="#932279"/><rect x="28" y="52" width="20" height="20" rx="2" fill="#EFA724"/><rect x="52" y="52" width="20" height="20" rx="2" fill="#fff"/></svg>"##;
const ALPINE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#0D597F"/><polygon points="50,24 78,72 22,72" fill="#fff"/></svg>"##;
const NIXOS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#5277C3"/><polygon points="50,22 74,36 74,64 50,78 26,64 26,36" fill="none" stroke="#fff" stroke-width="7"/></svg>"##;
const OPENSUSE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#73BA25"/><circle cx="50" cy="50" r="21" fill="none" stroke="#fff" stroke-width="8"/></svg>"##;
const APPLE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#27272a"/><path d="M62 48c0-9 7-13 7-13-4-6-10-6-12-6-5 0-10 3-12 3-3 0-7-3-11-3-6 0-11 4-14 9-6 11-2 27 4 35 3 4 6 9 11 9 4 0 6-3 11-3s6 3 11 3c5 0 8-4 11-9 3-5 4-10 4-10s-10-4-10-15z" fill="#fff"/></svg>"##;
const LINUX_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="#334155"/><circle cx="50" cy="36" r="14" fill="#facc15"/><ellipse cx="50" cy="64" rx="18" ry="18" fill="#f8fafc"/></svg>"##;

pub fn os_logo<'a, Message: 'a>(
    os: Option<&str>,
    server_name: &str,
    size: f32,
) -> Element<'a, Message> {
    if let Some(os_raw) = os {
        let key = os_raw.trim().to_ascii_lowercase();
        let raw_svg = match key.as_str() {
            "ubuntu" | "pop" | "pop!_os" => Some(UBUNTU_SVG),
            "debian" | "linuxmint" | "mint" | "kali" | "kali-linux" | "raspbian" => {
                Some(DEBIAN_SVG)
            }
            "arch" | "archlinux" | "manjaro" | "endeavouros" | "garuda" => Some(ARCH_SVG),
            "fedora" => Some(FEDORA_SVG),
            "centos" | "rocky" | "almalinux" | "alma" | "oracle" | "rhel" | "red hat" => {
                Some(CENTOS_SVG)
            }
            "alpine" => Some(ALPINE_SVG),
            "nixos" => Some(NIXOS_SVG),
            "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" | "suse" => Some(OPENSUSE_SVG),
            "darwin" | "macos" => Some(APPLE_SVG),
            "linux" | "gentoo" | "void" | "amazon" | "amzn" => Some(LINUX_SVG),
            _ => None,
        };

        if let Some(svg_str) = raw_svg {
            return svg(svg::Handle::from_memory(svg_str.as_bytes()))
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .into();
        }
        return initials_badge(os_raw, size);
    }

    initials_badge(server_name, size)
}

fn initials_badge<'a, Message: 'a>(label: &str, size: f32) -> Element<'a, Message> {
    let initials: String = label
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let txt = if initials.is_empty() {
        "??".to_string()
    } else {
        initials
    };

    container(
        text(txt)
            .size((size * 0.38).max(11.0))
            .color(BRAND_400)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .center_x(Length::Fixed(size))
    .center_y(Length::Fixed(size))
    .style(move |_| container::Style {
        background: Some(iced::Background::Color(DARK_700)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 999.0.into(),
        },
        ..Default::default()
    })
    .into()
}
