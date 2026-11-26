use crate::theme::{
    ghost_icon_button, logo_handle_for, pill_badge, rgba, titlebar_bg, truncate_ellipsis,
    BRAND_300, BRAND_400, FONT_MONO_MEDIUM, FONT_UI, FONT_UI_SEMIBOLD, RED_400, TEXT_DIM,
    TEXT_LIGHT, TEXT_MUTED, YELLOW_400,
};
#[cfg(not(target_os = "macos"))]
use crate::theme::{window_close_button, window_control_button};
use crate::ui::icons;
#[cfg(not(target_os = "macos"))]
use iced::mouse;
use iced::widget::{button, container, image, mouse_area, row, space, text};
#[cfg(not(target_os = "macos"))]
use iced::widget::{column, stack};
use iced::{alignment, window, Element, Length};
#[cfg(not(target_os = "macos"))]
use iced::{Background, Border};

#[derive(Debug, Clone)]
pub enum TitleBarMsg {
    Drag,
    Minimize,
    ToggleMaximize,
    Close,
    OpenGithub,
    ToggleLogs,
}

pub fn view<'a>(
    title: impl Into<std::borrow::Cow<'a, str>>,
    show_attribution: bool,
    unread_alerts: usize,
) -> Element<'a, TitleBarMsg> {
    let raw_title: std::borrow::Cow<'a, str> = title.into();
    let display_title = truncate_ellipsis(raw_title.as_ref(), 48);

    let mut left = row![];
    #[cfg(target_os = "macos")]
    {
        left = left.push(space().width(Length::Fixed(74.0)));
    }
    left = left
        .push(
            image(logo_handle_for(20))
                .width(20)
                .height(20)
                .filter_method(image::FilterMethod::Linear),
        )
        .push(
            text(display_title)
                .size(13)
                .font(FONT_UI_SEMIBOLD)
                .color(BRAND_300)
                .wrapping(text::Wrapping::None),
        )
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
        .on_press(TitleBarMsg::Drag)
        .on_double_click(TitleBarMsg::ToggleMaximize);

    let logs_color = if unread_alerts > 0 {
        YELLOW_400
    } else {
        TEXT_LIGHT
    };
    let mut logs_row = row![
        icons::scroll_text(13.0, logs_color),
        text("Logs")
            .size(11)
            .font(FONT_UI_SEMIBOLD)
            .color(logs_color),
    ]
    .spacing(5)
    .align_y(alignment::Vertical::Center);
    if unread_alerts > 0 {
        logs_row = logs_row.push(
            container(
                text(unread_alerts.to_string())
                    .size(10)
                    .font(FONT_MONO_MEDIUM)
                    .color(RED_400),
            )
            .padding([1, 5])
            .style(pill_badge(
                rgba(RED_400, 0.18),
                rgba(RED_400, 0.45),
                RED_400,
            )),
        );
    }
    let logs_btn = button(logs_row)
        .padding([4, 9])
        .style(ghost_icon_button)
        .on_press(TitleBarMsg::ToggleLogs);

    #[cfg(not(target_os = "macos"))]
    let controls: Element<'a, TitleBarMsg> = {
        let minimize_glyph = container(
            container(
                space()
                    .width(Length::Fixed(10.0))
                    .height(Length::Fixed(1.5)),
            )
            .style(|_| container::Style {
                background: Some(Background::Color(TEXT_LIGHT)),
                snap: true,
                ..Default::default()
            }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        let maximize_glyph = container(
            container(
                space()
                    .width(Length::Fixed(10.0))
                    .height(Length::Fixed(10.0)),
            )
            .style(|_| container::Style {
                border: Border {
                    color: TEXT_LIGHT,
                    width: 1.5,
                    radius: 0.0.into(),
                },
                snap: true,
                ..Default::default()
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

        row![
            container(logs_btn).padding([0, 4]),
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
        .align_y(alignment::Vertical::Center)
        .into()
    };

    #[cfg(target_os = "macos")]
    let controls: Element<'a, TitleBarMsg> = container(logs_btn).padding([0, 10]).into();

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

pub fn with_resize_handles<'a, M: Clone + 'a>(
    content: Element<'a, M>,
    on_resize: impl Fn(window::Direction) -> M,
) -> Element<'a, M> {
    #[cfg(target_os = "macos")]
    {
        let _ = on_resize;
        content
    }
    #[cfg(not(target_os = "macos"))]
    {
        let nw = mouse_area(
            space()
                .width(Length::Fixed(10.0))
                .height(Length::Fixed(4.0)),
        )
        .interaction(mouse::Interaction::ResizingDiagonallyDown)
        .on_press(on_resize(window::Direction::NorthWest));
        let n = mouse_area(space().width(Length::Fill).height(Length::Fixed(4.0)))
            .interaction(mouse::Interaction::ResizingVertically)
            .on_press(on_resize(window::Direction::North));
        let ne = mouse_area(
            space()
                .width(Length::Fixed(10.0))
                .height(Length::Fixed(4.0)),
        )
        .interaction(mouse::Interaction::ResizingDiagonallyUp)
        .on_press(on_resize(window::Direction::NorthEast));
        let top = row![nw, n, ne].width(Length::Fill);

        let w = mouse_area(space().width(Length::Fixed(5.0)).height(Length::Fill))
            .interaction(mouse::Interaction::ResizingHorizontally)
            .on_press(on_resize(window::Direction::West));
        let e = mouse_area(space().width(Length::Fixed(5.0)).height(Length::Fill))
            .interaction(mouse::Interaction::ResizingHorizontally)
            .on_press(on_resize(window::Direction::East));
        let middle = row![w, space().width(Length::Fill).height(Length::Fill), e]
            .width(Length::Fill)
            .height(Length::Fill);

        let sw = mouse_area(
            space()
                .width(Length::Fixed(10.0))
                .height(Length::Fixed(5.0)),
        )
        .interaction(mouse::Interaction::ResizingDiagonallyUp)
        .on_press(on_resize(window::Direction::SouthWest));
        let s = mouse_area(space().width(Length::Fill).height(Length::Fixed(5.0)))
            .interaction(mouse::Interaction::ResizingVertically)
            .on_press(on_resize(window::Direction::South));
        let se = mouse_area(
            space()
                .width(Length::Fixed(10.0))
                .height(Length::Fixed(5.0)),
        )
        .interaction(mouse::Interaction::ResizingDiagonallyDown)
        .on_press(on_resize(window::Direction::SouthEast));
        let bottom = row![sw, s, se].width(Length::Fill);

        stack![
            content,
            column![top, middle, bottom]
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

pub fn handle_titlebar_action<M>(id: window::Id, msg: TitleBarMsg) -> iced::Task<M> {
    match msg {
        TitleBarMsg::Drag => window::drag(id),
        TitleBarMsg::Minimize => window::minimize(id, true),
        TitleBarMsg::ToggleMaximize => window::toggle_maximize(id),
        TitleBarMsg::Close | TitleBarMsg::ToggleLogs => iced::Task::none(),
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
