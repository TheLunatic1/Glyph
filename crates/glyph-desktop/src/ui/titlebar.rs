use crate::theme::{
    ghost_icon_button, logo_handle, titlebar_bg, truncate_ellipsis, BRAND_300, BRAND_400,
    FONT_MONO_BOLD, RED_400, TEXT_DIM, TEXT_MUTED, TEXT_SECONDARY,
};
use crate::ui::icons;
use iced::widget::{button, container, image, mouse_area, row, space, text};
use iced::{alignment, window, Element, Length};

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
        image(logo_handle()).width(16).height(16),
        text(display_title)
            .size(12)
            .font(FONT_MONO_BOLD)
            .color(BRAND_300)
            .wrapping(text::Wrapping::None),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    if show_attribution {
        left = left
            .push(text("•").size(10).color(TEXT_DIM))
            .push(text("Made by").size(10).color(TEXT_MUTED))
            .push(
                button(
                    text("TheLunatic1 (Salman Toha)")
                        .size(10)
                        .color(BRAND_400)
                        .wrapping(text::Wrapping::None),
                )
                .padding([1, 3])
                .style(ghost_icon_button)
                .on_press(TitleBarMsg::OpenGithub),
            );
    }

    let drag_area = mouse_area(space().width(Length::Fill).height(Length::Fill))
        .on_press(TitleBarMsg::Drag);

    let controls = row![
        button(icons::minus(13.0, TEXT_SECONDARY))
            .padding([6, 12])
            .style(ghost_icon_button)
            .on_press(TitleBarMsg::Minimize),
        button(icons::maximize_box(12.0, TEXT_SECONDARY))
            .padding([6, 12])
            .style(ghost_icon_button)
            .on_press(TitleBarMsg::ToggleMaximize),
        button(icons::x_close(13.0, RED_400))
            .padding([6, 12])
            .style(ghost_icon_button)
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
        .height(Length::Fixed(32.0))
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(32.0))
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
