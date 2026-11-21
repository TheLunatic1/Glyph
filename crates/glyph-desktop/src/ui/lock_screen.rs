use crate::theme::{
    brand_button, dark_900_bg, dark_input, logo_handle, modal_card, rgba, BRAND_400,
    FONT_MONO_BOLD, RED_400, RED_500, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons;
use iced::widget::{button, column, container, image, row, space, text, text_input};
use iced::{alignment, Border, Color, Element, Length, Theme};

#[derive(Debug, Clone, Default)]
pub struct LockScreenState {
    pub input: String,
    pub error: Option<String>,
    pub idle_locked: bool,
}

#[derive(Debug, Clone)]
pub enum LockScreenMsg {
    InputChanged(String),
    Submit,
}

fn error_box(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(rgba(RED_500, 0.14))),
        border: Border {
            color: rgba(RED_500, 0.45),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(RED_400),
        snap: true,
        ..Default::default()
    }
}

pub fn view<'a>(st: &'a LockScreenState) -> Element<'a, LockScreenMsg> {
    let subtitle = if st.idle_locked {
        "Locked automatically after idle timeout."
    } else {
        "Enter your master passcode to unlock Glyph."
    };

    let mut card_col = column![
        row![
            image(logo_handle()).width(42).height(42),
            space().width(10),
            icons::lock_open(22.0, BRAND_400),
        ]
        .align_y(alignment::Vertical::Center),
        space().height(12),
        text("Glyph Vault Locked")
            .size(20)
            .font(FONT_MONO_BOLD)
            .color(TEXT_PRIMARY),
        space().height(4),
        text(subtitle).size(13).color(TEXT_SECONDARY),
        space().height(18),
        text_input("Enter master passcode...", &st.input)
            .secure(true)
            .on_input(LockScreenMsg::InputChanged)
            .on_submit(LockScreenMsg::Submit)
            .padding([10, 12])
            .size(14)
            .width(Length::Fill)
            .style(dark_input),
    ]
    .align_x(alignment::Horizontal::Center)
    .width(Length::Fill);

    if let Some(err) = &st.error {
        card_col = card_col.push(space().height(10)).push(
            container(text(err).size(12).color(RED_400))
                .padding([8, 10])
                .width(Length::Fill)
                .style(error_box),
        );
    }

    card_col = card_col.push(space().height(16)).push(
        button(
            container(
                text("Unlock Glyph")
                    .size(14)
                    .font(FONT_MONO_BOLD)
                    .color(Color::WHITE),
            )
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Center),
        )
        .on_press(LockScreenMsg::Submit)
        .padding([10, 16])
        .width(Length::Fill)
        .style(brand_button),
    );

    let card = container(card_col)
        .width(Length::Fixed(390.0))
        .padding(24)
        .style(modal_card);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(dark_900_bg)
        .into()
}
