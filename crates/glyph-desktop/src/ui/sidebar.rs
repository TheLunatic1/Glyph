use crate::theme::{
    danger_ghost_button, ghost_icon_button, logo_handle_for, mobile_pill_button, sidebar_bg,
    sidebar_nav_button, APP_VERSION, BRAND_300, BRAND_400, FONT_MONO_MEDIUM, FONT_UI_BOLD,
    FONT_UI_SEMIBOLD, RED_400, TEXT_DIM, TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons;
use iced::widget::{button, column, container, image, row, scrollable, space, text};
use iced::{alignment, Element, Length, Padding};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerTab {
    Dashboard,
    Terminal,
    Sftp,
    Commands,
    Secrets,
    Tunnels,
    Containers,
    Vnc,
}

impl ServerTab {
    pub const ALL: [(Self, &'static str); 8] = [
        (Self::Dashboard, "Dashboard"),
        (Self::Terminal, "Terminal"),
        (Self::Sftp, "SFTP"),
        (Self::Commands, "Commands"),
        (Self::Secrets, "Secrets"),
        (Self::Tunnels, "Tunnels"),
        (Self::Containers, "Containers"),
        (Self::Vnc, "VNC"),
    ];
}

#[derive(Debug, Clone)]
pub enum SidebarMsg {
    SelectTab(ServerTab),
    OpenMobile,
    Disconnect,
    OpenGithub,
}

pub fn view<'a>(active_tab: ServerTab) -> Element<'a, SidebarMsg> {
    let brand_header = container(
        row![
            image(logo_handle_for(36))
                .width(36)
                .height(36)
                .filter_method(image::FilterMethod::Linear),
            text("Glyph")
                .size(20)
                .font(FONT_UI_BOLD)
                .color(TEXT_PRIMARY),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center),
    )
    .padding([0, 24]);

    let mut nav_col = column![].spacing(4).padding([0, 16]);
    for (tab, label) in ServerTab::ALL {
        let is_active = active_tab == tab;
        let icon_color = if is_active { BRAND_400 } else { TEXT_SECONDARY };
        let icon: Element<'a, SidebarMsg> = match tab {
            ServerTab::Dashboard => icons::activity(20.0, icon_color).into(),
            ServerTab::Terminal => icons::terminal(20.0, icon_color).into(),
            ServerTab::Sftp => icons::folder_open(20.0, icon_color).into(),
            ServerTab::Commands => icons::code(20.0, icon_color).into(),
            ServerTab::Secrets => icons::lock(20.0, icon_color).into(),
            ServerTab::Tunnels => icons::network(20.0, icon_color).into(),
            ServerTab::Containers => icons::cube(20.0, icon_color).into(),
            ServerTab::Vnc => icons::monitor(20.0, icon_color).into(),
        };

        let btn = button(
            row![
                icon,
                text(label)
                    .size(14)
                    .font(if is_active { FONT_UI_SEMIBOLD } else { crate::theme::FONT_UI })
                    .color(if is_active { BRAND_400 } else { TEXT_LIGHT }),
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding([11, 16])
        .style(sidebar_nav_button(is_active))
        .on_press(SidebarMsg::SelectTab(tab));

        nav_col = nav_col.push(btn);
    }

    let mobile_btn = button(
        row![
            icons::smartphone(16.0, BRAND_400),
            text("Glyph Mobile (APK)").size(12).color(BRAND_300),
            space().width(Length::Fill),
            icons::qr_code(13.0, TEXT_MUTED),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding([10, 14])
    .style(mobile_pill_button)
    .on_press(SidebarMsg::OpenMobile);

    let disconnect_btn = button(
        row![
            icons::log_out(20.0, RED_400),
            text("Disconnect").size(14).color(RED_400),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding([11, 16])
    .style(danger_ghost_button)
    .on_press(SidebarMsg::Disconnect);

    let attribution = column![
        row![
            text("Made by ").size(11).color(TEXT_MUTED),
            button(text("TheLunatic1").size(11).color(BRAND_400))
                .padding([0, 2])
                .style(ghost_icon_button)
                .on_press(SidebarMsg::OpenGithub),
        ]
        .align_y(alignment::Vertical::Center),
        text("(Salman Toha)").size(11).color(BRAND_400),
        text(format!("v{APP_VERSION}"))
            .size(11)
            .font(FONT_MONO_MEDIUM)
            .color(TEXT_DIM),
    ]
    .spacing(2)
    .align_x(alignment::Horizontal::Center)
    .width(Length::Fill);

    let footer = container(column![mobile_btn, disconnect_btn, attribution].spacing(10))
        .padding([0, 16])
        .width(Length::Fill);

    container(
        column![
            brand_header,
            space().height(28),
            scrollable(nav_col).height(Length::Fill),
            footer,
        ]
        .padding(Padding {
            top: 24.0,
            right: 0.0,
            bottom: 16.0,
            left: 0.0,
        })
        .height(Length::Fill),
    )
    .width(Length::Fixed(256.0))
    .height(Length::Fill)
    .style(sidebar_bg)
    .into()
}
