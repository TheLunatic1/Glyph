use crate::theme::{
    ghost_icon_button, logo_handle_for, titlebar_bg, truncate_ellipsis, window_close_button,
    window_control_button, BRAND_300, BRAND_400, FONT_UI, FONT_UI_SEMIBOLD, TEXT_DIM, TEXT_LIGHT,
    TEXT_MUTED,
};
use crate::ui::icons;
use iced::widget::{button, container, image, mouse_area, row, space, text};
use iced::{alignment, window, Background, Border, Element, Length};

#[derive(Debug, Clone)]
pub enum TitleBarMsg {
    Drag,
    Minimize,
    ToggleMaximize,
    Close,
    OpenGithub,
}

pub fn view<'a>(
    title: impl Into<std::borrow::Cow<'a, str>>,
    show_attribution: bool,
) -> Element<'a, TitleBarMsg> {
    let raw_title: std::borrow::Cow<'a, str> = title.into();
    let display_title = truncate_ellipsis(raw_title.as_ref(), 48);

    let mut left = row![
        image(logo_handle_for(20))
            .width(20)
            .height(20)
            .filter_method(image::FilterMethod::Linear),
        text(display_title)
            .size(13)
            .font(FONT_UI_SEMIBOLD)
            .color(BRAND_300)
            .wrapping(text::Wrapping::None),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    if show_attribution {
        left = left
            .push(text("·").size(11).font(FONT_UI).color(TEXT_DIM))
            .push(text("Made by").size(11).font(FONT_UI).color(TEXT_MUTED))
            .push(
                button(
                    text("TheLunatic1 (Salman Toha)")
                        .size(11)
                        .font(FONT_UI)
                        .color(BRAND_400)
                        .wrapping(text::Wrapping::None),
                )
                .padding([1, 4])
                .style(ghost_icon_button)
                .on_press(TitleBarMsg::OpenGithub),
            );
    }

    let drag_area = mouse_area(space().width(Length::Fill).height(Length::Fill))
        .on_press(TitleBarMsg::Drag);

    let minimize_glyph = container(
        container(space().width(Length::Fixed(10.0)).height(Length::Fixed(1.5))).style(|_| {
            container::Style {
                background: Some(Background::Color(TEXT_LIGHT)),
                snap: true,
                ..Default::default()
            }
        }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill);

    let maximize_glyph = container(
        container(space().width(Length::Fixed(10.0)).height(Length::Fixed(10.0))).style(|_| {
            container::Style {
                border: Border {
                    color: TEXT_LIGHT,
                    width: 1.5,
                    radius: 0.0.into(),
                },
                snap: true,
                ..Default::default()
            }
        }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill);

    let close_glyph = container(icons::x_close_inherit(15.0))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    let controls = row![
        button(minimize_glyph)
            .width(Length::Fixed(44.0))
            .height(Length::Fixed(34.0))
            .padding(0)
            .style(window_control_button)
            .on_press(TitleBarMsg::Minimize),
        button(maximize_glyph)
            .width(Length::Fixed(44.0))
            .height(Length::Fixed(34.0))
            .padding(0)
            .style(window_control_button)
            .on_press(TitleBarMsg::ToggleMaximize),
        button(close_glyph)
            .width(Length::Fixed(44.0))
            .height(Length::Fixed(34.0))
            .padding(0)
            .style(window_close_button)
            .on_press(TitleBarMsg::Close),
    ]
    .align_y(alignment::Vertical::Center);

    container(
        row![
            container(left).padding([0, 12]).clip(true),
            drag_area,
            controls
        ]
        .width(Length::Fill)
        .height(Length::Fixed(34.0))
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(34.0))
    .style(titlebar_bg)
    .into()
}

pub fn handle_titlebar_action<M>(id: window::Id, msg: TitleBarMsg) -> iced::Task<M> {
    match msg {
        TitleBarMsg::Drag => window::drag(id),
        TitleBarMsg::Minimize => window::minimize(id, true),
        TitleBarMsg::ToggleMaximize => window::toggle_maximize(id),
        TitleBarMsg::Close => iced::Task::none(), // caller handles hide-to-tray vs close
        TitleBarMsg::OpenGithub => {
            open_url_in_browser("https://github.com/TheLunatic1");
            iced::Task::none()
        }
    }
}

pub fn open_url_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}
