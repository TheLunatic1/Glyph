use crate::theme::{
    brand_button, danger_button, dark_card, dark_input, dark_scrollable, glass_panel, pill_badge,
    rgba, secondary_button, BRAND_300, BRAND_400, BRAND_500, DARK_700, FONT_MONO, FONT_MONO_BOLD,
    GREEN_400, GREEN_500, RED_400, TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons;
use glyph_vnc::{vnc_viewport, VncClientHandle, VncFrame, VncUiEvent};
use iced::widget::{
    button, column, container, row, scrollable, space, text, text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct VncPageState {
    pub host: String,
    pub port: String,
    pub password: String,
    pub show_password: bool,
    pub connecting: bool,
    pub connected: bool,
    pub desktop_name: String,
    pub width: u16,
    pub height: u16,
    pub fit_to_window: bool,
    pub focused: bool,
    pub frame: Option<Arc<VncFrame>>,
    pub handle: Option<VncClientHandle>,
    pub server_clipboard: Option<String>,
    pub error: Option<String>,
}

impl Default for VncPageState {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: "5900".to_string(),
            password: String::new(),
            show_password: false,
            connecting: false,
            connected: false,
            desktop_name: String::new(),
            width: 0,
            height: 0,
            fit_to_window: true,
            focused: true,
            frame: None,
            handle: None,
            server_clipboard: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum VncPageMsg {
    HostChanged(String),
    PortChanged(String),
    PasswordChanged(String),
    ToggleShowPassword,
    Connect,
    Disconnect,
    ToggleFit,
    SendCtrlAltDel,
    SendLocalClipboard,
    CopyRemoteClipboard,
    RefreshFrame,
    Viewport(VncUiEvent),
}

pub fn view<'a>(st: &'a VncPageState) -> Element<'a, VncPageMsg> {
    let header = row![
        container(icons::monitor(20.0, BRAND_400))
            .padding(10)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.14))),
                border: Border {
                    color: rgba(BRAND_500, 0.3),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                snap: true,
                ..Default::default()
            }),
        column![
            text("VNC Remote Desktop (over SSH)")
                .size(20)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("RFB 3.8 encrypted end-to-end inside SSH direct-tcpip")
                .size(12)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        if st.connected {
            container(
                text(format!(
                    "{}  ({}x{})",
                    if st.desktop_name.is_empty() {
                        "Connected"
                    } else {
                        st.desktop_name.as_str()
                    },
                    st.width,
                    st.height
                ))
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(GREEN_400),
            )
            .padding([4, 10])
            .style(pill_badge(
                rgba(GREEN_500, 0.14),
                rgba(GREEN_500, 0.35),
                GREEN_400,
            ))
        } else {
            container(text("Disconnected").size(11).color(TEXT_MUTED))
                .padding([4, 10])
                .style(pill_badge(DARK_700, DARK_700, TEXT_MUTED))
        },
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let connect_bar = if st.connected {
        container(
            row![
                button(
                    row![
                        icons::maximize_2(13.0, TEXT_LIGHT),
                        text(if st.fit_to_window {
                            "Fit: ON"
                        } else {
                            "Scale: 1:1"
                        })
                        .size(12)
                        .color(TEXT_LIGHT),
                    ]
                    .spacing(5)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([7, 12])
                .style(secondary_button)
                .on_press(VncPageMsg::ToggleFit),
                button(
                    row![
                        icons::keyboard(13.0, TEXT_LIGHT),
                        text("Ctrl+Alt+Del").size(12).color(TEXT_LIGHT),
                    ]
                    .spacing(5)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([7, 12])
                .style(secondary_button)
                .on_press(VncPageMsg::SendCtrlAltDel),
                button(
                    row![
                        icons::clipboard(13.0, TEXT_LIGHT),
                        text("Paste Clipboard").size(12).color(TEXT_LIGHT),
                    ]
                    .spacing(5)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([7, 12])
                .style(secondary_button)
                .on_press(VncPageMsg::SendLocalClipboard),
                button(icons::refresh_cw(14.0, TEXT_LIGHT))
                    .padding([7, 10])
                    .style(secondary_button)
                    .on_press(VncPageMsg::RefreshFrame),
                space().width(Length::Fill),
                button(
                    row![
                        icons::unlink(14.0, Color::WHITE),
                        text("Disconnect")
                            .size(12)
                            .font(FONT_MONO_BOLD)
                            .color(Color::WHITE),
                    ]
                    .spacing(6)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([7, 14])
                .style(danger_button)
                .on_press(VncPageMsg::Disconnect),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding(12)
        .style(glass_panel)
    } else {
        let connect_btn = button(
            row![
                icons::plug(14.0, Color::WHITE),
                text(if st.connecting {
                    "Connecting..."
                } else {
                    "Connect VNC"
                })
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(Color::WHITE),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([9, 16])
        .style(brand_button);

        container(
            row![
                text_input("Target Host (127.0.0.1)", &st.host)
                    .on_input(VncPageMsg::HostChanged)
                    .padding([8, 10])
                    .size(13)
                    .font(FONT_MONO)
                    .width(Length::FillPortion(2))
                    .style(dark_input),
                text_input("Port (5900)", &st.port)
                    .on_input(VncPageMsg::PortChanged)
                    .padding([8, 10])
                    .size(13)
                    .font(FONT_MONO)
                    .width(Length::Fixed(105.0))
                    .style(dark_input),
                text_input("VNC Password (optional)", &st.password)
                    .secure(!st.show_password)
                    .on_input(VncPageMsg::PasswordChanged)
                    .on_submit(VncPageMsg::Connect)
                    .padding([8, 10])
                    .size(13)
                    .width(Length::FillPortion(2))
                    .style(dark_input),
                button(if st.show_password {
                    icons::eye_off(14.0, TEXT_SECONDARY)
                } else {
                    icons::eye(14.0, TEXT_SECONDARY)
                })
                .padding([8, 10])
                .style(secondary_button)
                .on_press(VncPageMsg::ToggleShowPassword),
                if st.connecting {
                    connect_btn
                } else {
                    connect_btn.on_press(VncPageMsg::Connect)
                },
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding(12)
        .style(glass_panel)
    };

    let mut body = column![header, connect_bar].spacing(12).padding(20);

    if let Some(err) = &st.error {
        body = body.push(
            container(text(err).size(12).color(RED_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
        );
    }

    if let Some(clip) = &st.server_clipboard {
        if !clip.is_empty() {
            body = body.push(
                container(
                    row![
                        icons::clipboard(13.0, BRAND_300),
                        text("Remote clipboard received")
                            .size(12)
                            .color(TEXT_SECONDARY),
                        space().width(Length::Fill),
                        button(text("Copy to Local").size(11).color(BRAND_300))
                            .padding([4, 10])
                            .style(secondary_button)
                            .on_press(VncPageMsg::CopyRemoteClipboard),
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                )
                .width(Length::Fill)
                .padding([6, 12])
                .style(dark_card),
            );
        }
    }

    let viewport_box: Element<'a, VncPageMsg> = if let Some(frame) = &st.frame {
        let canvas_el = vnc_viewport(frame, st.fit_to_window, st.focused).map(VncPageMsg::Viewport);
        if st.fit_to_window {
            container(canvas_el)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(dark_card)
                .into()
        } else {
            container(
                scrollable(canvas_el)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(dark_scrollable),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(dark_card)
            .into()
        }
    } else {
        container(
            column![
                icons::monitor(42.0, TEXT_MUTED),
                text("No Active VNC Display")
                    .size(15)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                text("Start a VNC server on the remote machine (e.g. `x11vnc -localhost -display :0` or `vncserver :1`), then click Connect VNC.")
                    .size(12)
                    .color(TEXT_SECONDARY),
            ]
            .spacing(8)
            .align_x(alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(32)
        .style(dark_card)
        .into()
    };

    body = body.push(viewport_box);
    body.width(Length::Fill).height(Length::Fill).into()
}
