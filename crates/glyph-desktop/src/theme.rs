use iced::font::{Family, Weight};
use iced::theme::Palette;
use iced::widget::{button, container, image, pick_list, scrollable, text_editor, text_input};
use iced::{color, Background, Border, Color, Font, Shadow, Theme, Vector};
use std::sync::OnceLock;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const LOGO_PNG: &[u8] = include_bytes!("../../../assets/logo.png");
pub const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");
pub const JETBRAINS_MONO_BOLD: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMono-Bold.ttf");

pub const FONT_MONO: Font = Font {
    family: Family::Name("JetBrains Mono"),
    weight: Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const FONT_MONO_BOLD: Font = Font {
    family: Family::Name("JetBrains Mono"),
    weight: Weight::Bold,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

// Exact Glyph Tailwind Palette
pub const DARK_900: Color = color!(0x0f111a);
pub const DARK_800: Color = color!(0x1a1d2d);
pub const DARK_700: Color = color!(0x262a40);
pub const DARK_600: Color = color!(0x374151);

pub const BRAND_500: Color = color!(0x6366f1);
pub const BRAND_400: Color = color!(0x818cf8);
pub const BRAND_300: Color = color!(0xa5b4fc);

pub const TEXT_PRIMARY: Color = color!(0xf3f4f6);
pub const TEXT_LIGHT: Color = color!(0xe5e7eb);
pub const TEXT_SECONDARY: Color = color!(0x9ca3af);
pub const TEXT_MUTED: Color = color!(0x6b7280);
pub const TEXT_DIM: Color = color!(0x4b5563);

pub const GREEN_400: Color = color!(0x4ade80);
pub const GREEN_500: Color = color!(0x22c55e);
pub const RED_400: Color = color!(0xf87171);
pub const RED_500: Color = color!(0xef4444);
pub const RED_600: Color = color!(0xdc2626);
pub const YELLOW_400: Color = color!(0xfacc15);
pub const YELLOW_500: Color = color!(0xf59e0b);
pub const BLUE_400: Color = color!(0x60a5fa);
pub const ORANGE_400: Color = color!(0xfb923c);
pub const PURPLE_400: Color = color!(0xc084fc);
pub const CYAN_400: Color = color!(0x22d3ee);

static CACHED_LOGO: OnceLock<image::Handle> = OnceLock::new();

pub fn logo_handle() -> image::Handle {
    CACHED_LOGO
        .get_or_init(|| image::Handle::from_bytes(LOGO_PNG))
        .clone()
}

pub fn truncate_ellipsis(s: &str, max_chars: usize) -> String {
    if max_chars <= 1 {
        return "…".to_string();
    }
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars - 1).collect();
    out.push('…');
    out
}

pub fn glyph_theme() -> Theme {
    Theme::custom(
        "Glyph",
        Palette {
            background: DARK_900,
            text: TEXT_PRIMARY,
            primary: BRAND_500,
            success: GREEN_500,
            warning: YELLOW_500,
            danger: RED_500,
        },
    )
}

pub fn rgba(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

pub fn dark_900_bg(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DARK_900)),
        text_color: Some(TEXT_PRIMARY),
        snap: true,
        ..Default::default()
    }
}

pub fn dark_800_bg(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DARK_800)),
        text_color: Some(TEXT_PRIMARY),
        snap: true,
        ..Default::default()
    }
}

pub fn glass_panel(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DARK_800)),
        text_color: Some(TEXT_PRIMARY),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: rgba(Color::BLACK, 0.20),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 4.0,
        },
        snap: true,
    }
}

/// Crisp, zero-bleed card surface for dense multi-column grids.
#[allow(dead_code)]
pub fn grid_card(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DARK_800)),
        text_color: Some(TEXT_PRIMARY),
        border: Border {
            color: Color::from_rgb8(42, 47, 71),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn dark_card(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DARK_900)),
        text_color: Some(TEXT_PRIMARY),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn modal_backdrop(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(rgba(Color::BLACK, 0.68))),
        snap: true,
        ..Default::default()
    }
}

pub fn modal_card(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DARK_900)),
        text_color: Some(TEXT_PRIMARY),
        border: Border {
            color: Color::from_rgb8(53, 59, 87),
            width: 1.0,
            radius: 14.0.into(),
        },
        shadow: Shadow {
            color: rgba(Color::BLACK, 0.45),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 14.0,
        },
        snap: true,
    }
}

pub fn titlebar_bg(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DARK_900)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 0.0.into(),
        },
        snap: true,
        ..Default::default()
    }
}

pub fn sidebar_bg(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 0.0.into(),
        },
        snap: true,
        ..Default::default()
    }
}

pub fn brand_button(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => BRAND_400,
        button::Status::Disabled => rgba(BRAND_500, 0.45),
        button::Status::Active => BRAND_500,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: if matches!(status, button::Status::Disabled) {
            rgba(Color::WHITE, 0.6)
        } else {
            Color::WHITE
        },
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: rgba(BRAND_300, 0.22),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn secondary_button(_: &Theme, status: button::Status) -> button::Style {
    let (bg, border_c) = match status {
        button::Status::Hovered | button::Status::Pressed => {
            (DARK_600, Color::from_rgb8(75, 85, 110))
        }
        button::Status::Disabled => (rgba(DARK_700, 0.5), DARK_700),
        button::Status::Active => (DARK_700, Color::from_rgb8(54, 60, 88)),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_LIGHT,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: border_c,
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn ghost_icon_button(_: &Theme, status: button::Status) -> button::Style {
    let (bg, txt) = match status {
        button::Status::Hovered | button::Status::Pressed => (rgba(BRAND_500, 0.16), BRAND_400),
        _ => (Color::TRANSPARENT, TEXT_SECONDARY),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: txt,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn danger_button(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => RED_600,
        button::Status::Disabled => rgba(RED_500, 0.45),
        button::Status::Active => RED_500,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: rgba(RED_400, 0.3),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn danger_ghost_button(_: &Theme, status: button::Status) -> button::Style {
    let (bg, txt) = match status {
        button::Status::Hovered | button::Status::Pressed => (rgba(RED_500, 0.16), RED_400),
        _ => (Color::TRANSPARENT, RED_400),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: txt,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn mobile_pill_button(_: &Theme, status: button::Status) -> button::Style {
    let alpha = match status {
        button::Status::Hovered | button::Status::Pressed => 0.26,
        _ => 0.15,
    };
    button::Style {
        background: Some(Background::Color(rgba(BRAND_500, alpha))),
        text_color: BRAND_300,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: rgba(BRAND_500, 0.38),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn server_card_button(_: &Theme, status: button::Status) -> button::Style {
    let (bg, border_color) = match status {
        button::Status::Hovered | button::Status::Pressed => {
            (Color::from_rgb8(31, 35, 55), rgba(BRAND_500, 0.72))
        }
        _ => (DARK_800, Color::from_rgb8(42, 47, 71)),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn sidebar_nav_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        if active {
            button::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.16))),
                text_color: BRAND_400,
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: rgba(BRAND_500, 0.28),
                },
                shadow: Shadow::default(),
                snap: true,
            }
        } else {
            let (bg, txt) = match status {
                button::Status::Hovered | button::Status::Pressed => (DARK_700, TEXT_LIGHT),
                _ => (Color::TRANSPARENT, TEXT_SECONDARY),
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: txt,
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                shadow: Shadow::default(),
                snap: true,
            }
        }
    }
}

pub fn dark_input(_: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => BRAND_500,
        text_input::Status::Hovered => DARK_600,
        _ => DARK_700,
    };
    text_input::Style {
        background: Background::Color(DARK_900),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: TEXT_MUTED,
        placeholder: TEXT_DIM,
        value: TEXT_PRIMARY,
        selection: rgba(BRAND_500, 0.35),
    }
}

pub fn dark_pick_list(_: &Theme, status: pick_list::Status) -> pick_list::Style {
    let border_color = match status {
        pick_list::Status::Opened { .. } => BRAND_500,
        pick_list::Status::Hovered => DARK_600,
        pick_list::Status::Active => DARK_700,
    };
    pick_list::Style {
        text_color: TEXT_PRIMARY,
        placeholder_color: TEXT_MUTED,
        handle_color: TEXT_SECONDARY,
        background: Background::Color(DARK_900),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
    }
}

pub fn dark_scrollable(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let mut base = scrollable::default(theme, status);
    let thumb = match status {
        scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. } => DARK_600,
        _ => DARK_700,
    };
    base.vertical_rail.background = Some(Background::Color(Color::TRANSPARENT));
    base.vertical_rail.border = Border::default();
    base.vertical_rail.scroller.background = Background::Color(thumb);
    base.vertical_rail.scroller.border = Border {
        radius: 4.0.into(),
        ..Default::default()
    };
    base.horizontal_rail.background = Some(Background::Color(Color::TRANSPARENT));
    base.horizontal_rail.border = Border::default();
    base.horizontal_rail.scroller.background = Background::Color(thumb);
    base.horizontal_rail.scroller.border = Border {
        radius: 4.0.into(),
        ..Default::default()
    };
    base
}

pub fn pill_badge(bg: Color, border: Color, fg: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(bg)),
        text_color: Some(fg),
        border: Border {
            color: border,
            width: 1.0,
            radius: 999.0.into(),
        },
        snap: true,
        ..Default::default()
    }
}

pub fn dark_text_editor(_: &Theme, status: text_editor::Status) -> text_editor::Style {
    let border_color = match status {
        text_editor::Status::Focused { .. } => BRAND_500,
        text_editor::Status::Hovered => DARK_600,
        _ => DARK_700,
    };
    text_editor::Style {
        background: Background::Color(DARK_900),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        placeholder: TEXT_DIM,
        value: TEXT_LIGHT,
        selection: rgba(BRAND_500, 0.35),
    }
}
