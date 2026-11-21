use crate::theme::{
    dark_900_bg, rgba, BRAND_400, BRAND_500, FONT_MONO_BOLD, LOGO_PNG, TEXT_MUTED, TEXT_PRIMARY,
};
use iced::widget::{column, container, image, row, text};
use iced::{alignment, Border, Element, Length};

pub fn view<'a, Message: 'a>() -> Element<'a, Message> {
    let logo = container(image(image::Handle::from_bytes(LOGO_PNG)).width(80).height(80))
        .padding(8)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(rgba(BRAND_500, 0.08))),
            border: Border {
                color: rgba(BRAND_500, 0.18),
                width: 1.0,
                radius: 20.0.into(),
            },
            ..Default::default()
        });

    let dots = row![dot(), dot(), dot()].spacing(6);

    let inner = column![
        logo,
        text("Glyph")
            .size(36)
            .font(FONT_MONO_BOLD)
            .color(TEXT_PRIMARY),
        text("Secure SSH & Server Management")
            .size(12)
            .color(TEXT_MUTED),
        dots,
    ]
    .spacing(14)
    .align_x(alignment::Horizontal::Center);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(dark_900_bg)
        .into()
}

fn dot<'a, Message: 'a>() -> Element<'a, Message> {
    container(text(""))
        .width(Length::Fixed(6.0))
        .height(Length::Fixed(6.0))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(BRAND_400)),
            border: Border {
                radius: 999.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
