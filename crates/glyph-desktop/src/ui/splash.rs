use crate::theme::{
    logo_handle, rgba, APP_VERSION, BRAND_300, BRAND_400, BRAND_500, DARK_700, DARK_800, DARK_900,
    FONT_MONO, FONT_MONO_BOLD, TEXT_MUTED, TEXT_PRIMARY,
};
use iced::widget::{column, container, image, row, space, text};
use iced::{alignment, Background, Border, Color, Element, Length};

pub fn view<'a, Message: 'a>(elapsed_ms: u64, alpha: f32) -> Element<'a, Message> {
    let a = alpha.clamp(0.0, 1.0);
    let pulse = ((elapsed_ms as f32 / 180.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);

    let logo = container(
        image(logo_handle())
            .width(88)
            .height(88)
            .filter_method(image::FilterMethod::Linear),
    )
    .padding(14)
    .style(move |_| container::Style {
        background: Some(Background::Color(rgba(DARK_800, 0.92 * a))),
        border: Border {
            color: rgba(BRAND_500, (0.28 + 0.28 * pulse) * a),
            width: 1.5,
            radius: 22.0.into(),
        },
        snap: true,
        ..Default::default()
    });

    let dots = row![
        anim_dot(elapsed_ms, 0, a),
        anim_dot(elapsed_ms, 180, a),
        anim_dot(elapsed_ms, 360, a),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    let pct = ((elapsed_ms as f32 / 1150.0).clamp(0.08, 1.0) * 100.0).round() as u16;
    let fill_p = pct.clamp(8, 100);
    let rem_p = 100u16.saturating_sub(fill_p);

    let mut progress_row = row![container(space().width(Length::Fill).height(3.0))
        .width(Length::FillPortion(fill_p))
        .style(move |_| container::Style {
            background: Some(Background::Color(rgba(BRAND_500, a))),
            border: Border {
                radius: 999.0.into(),
                ..Default::default()
            },
            snap: true,
            ..Default::default()
        })];
    if rem_p > 0 {
        progress_row = progress_row.push(space().width(Length::FillPortion(rem_p)));
    }

    let progress_track = container(progress_row)
        .width(Length::Fixed(180.0))
        .height(Length::Fixed(3.0))
        .style(move |_| container::Style {
            background: Some(Background::Color(rgba(DARK_700, a))),
            border: Border {
                radius: 999.0.into(),
                ..Default::default()
            },
            snap: true,
            ..Default::default()
        });

    let version_pill = container(
        text(format!("v{APP_VERSION}"))
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(rgba(BRAND_300, a)),
    )
    .padding([3, 10])
    .style(move |_| container::Style {
        background: Some(Background::Color(rgba(BRAND_500, 0.14 * a))),
        border: Border {
            color: rgba(BRAND_500, 0.35 * a),
            width: 1.0,
            radius: 999.0.into(),
        },
        snap: true,
        ..Default::default()
    });

    let inner = column![
        logo,
        row![
            text("Glyph")
                .size(38)
                .font(FONT_MONO_BOLD)
                .color(rgba(TEXT_PRIMARY, a)),
            version_pill,
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
        text("Secure SSH & Server Management")
            .size(13)
            .font(FONT_MONO)
            .color(rgba(TEXT_MUTED, a)),
        progress_track,
        dots,
        text("Made by TheLunatic1 (Salman Toha)")
            .size(11)
            .color(rgba(Color::from_rgb8(90, 99, 128), a)),
    ]
    .spacing(14)
    .align_x(alignment::Horizontal::Center);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(rgba(DARK_900, a))),
            text_color: Some(rgba(TEXT_PRIMARY, a)),
            snap: true,
            ..Default::default()
        })
        .into()
}

fn anim_dot<'a, Message: 'a>(elapsed_ms: u64, phase_ms: u64, alpha: f32) -> Element<'a, Message> {
    let t = (elapsed_ms + phase_ms) as f32 / 170.0;
    let wave = (t.sin() * 0.5 + 0.5).clamp(0.25, 1.0);
    let diameter = 6.0 + wave * 2.5;
    container(text(""))
        .width(Length::Fixed(diameter))
        .height(Length::Fixed(diameter))
        .style(move |_| container::Style {
            background: Some(Background::Color(rgba(BRAND_400, wave * alpha))),
            border: Border {
                radius: 999.0.into(),
                ..Default::default()
            },
            snap: true,
            ..Default::default()
        })
        .into()
}
