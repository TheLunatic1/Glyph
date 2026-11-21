use crate::theme::{
    brand_button, danger_ghost_button, dark_card, dark_input, dark_pick_list, dark_scrollable,
    glass_panel, pill_badge, rgba, secondary_button, BRAND_300, BRAND_400, BRAND_500, DARK_700,
    DARK_800, DARK_900, FONT_MONO, FONT_MONO_BOLD, GREEN_400, GREEN_500, RED_400, TEXT_LIGHT,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, BLUE_400,
};
use crate::ui::icons;
use glyph_core::tunnels::{TunnelDirection, TunnelInfo};
use iced::widget::{
    button, column, container, pick_list, row, scrollable, space, text, text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiTunnelDirection {
    Local,
    Remote,
}

#[derive(Debug, Clone)]
pub struct TunnelsPageState {
    pub direction: UiTunnelDirection,
    pub bind_port: String,
    pub target_host: String,
    pub target_port: String,
    pub protocol: String,
    pub tunnels: Vec<TunnelInfo>,
    pub schemes: HashMap<String, bool>, // true = https, false = http
    pub busy: bool,
    pub error: Option<String>,
}

impl Default for TunnelsPageState {
    fn default() -> Self {
        Self {
            direction: UiTunnelDirection::Local,
            bind_port: String::new(),
            target_host: "127.0.0.1".to_string(),
            target_port: String::new(),
            protocol: "TCP".to_string(),
            tunnels: Vec::new(),
            schemes: HashMap::new(),
            busy: false,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TunnelsPageMsg {
    SelectDirection(UiTunnelDirection),
    BindPortChanged(String),
    TargetHostChanged(String),
    TargetPortChanged(String),
    ProtocolSelected(String),
    BindSubmit,
    ToggleScheme(String),
    OpenUrl(String),
    StopTunnel(String),
    Refresh,
}

pub fn view<'a>(st: &'a TunnelsPageState) -> Element<'a, TunnelsPageMsg> {
    let header = row![
        container(icons::network(20.0, BRAND_400))
            .padding(10)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.14))),
                border: Border {
                    color: rgba(BRAND_500, 0.3),
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }),
        column![
            text("Port Forwarding")
                .size(20)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("Encrypted Local (-L) and Remote (-R) SSH tunnels")
                .size(12)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        button(
            row![
                icons::refresh_cw(14.0, TEXT_LIGHT),
                text("Refresh").size(12).color(TEXT_LIGHT),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 12])
        .style(secondary_button)
        .on_press(TunnelsPageMsg::Refresh),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let is_local = st.direction == UiTunnelDirection::Local;
    let dir_pills = row![
        button(
            text("Local Forward (-L)")
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(if is_local { Color::WHITE } else { TEXT_SECONDARY }),
        )
        .padding([6, 12])
        .style(move |_, _| {
            if is_local {
                brand_button(&crate::theme::glyph_theme(), button::Status::Active)
            } else {
                secondary_button(&crate::theme::glyph_theme(), button::Status::Active)
            }
        })
        .on_press(TunnelsPageMsg::SelectDirection(UiTunnelDirection::Local)),
        button(
            text("Remote Forward (-R)")
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(if !is_local { Color::WHITE } else { TEXT_SECONDARY }),
        )
        .padding([6, 12])
        .style(move |_, _| {
            if !is_local {
                brand_button(&crate::theme::glyph_theme(), button::Status::Active)
            } else {
                secondary_button(&crate::theme::glyph_theme(), button::Status::Active)
            }
        })
        .on_press(TunnelsPageMsg::SelectDirection(UiTunnelDirection::Remote)),
    ]
    .spacing(8);

    let bind_placeholder = if is_local {
        "Local Port (e.g. 8080, 0=auto)"
    } else {
        "Remote Listen Port (e.g. 9000)"
    };
    let host_placeholder = if is_local {
        "Remote Host (127.0.0.1)"
    } else {
        "Local Host (127.0.0.1)"
    };
    let target_placeholder = if is_local {
        "Remote Port (e.g. 80)"
    } else {
        "Local Port (e.g. 3000)"
    };

    let protocols = vec!["TCP".to_string(), "UDP".to_string()];

    let mut form_row = row![
        text_input(bind_placeholder, &st.bind_port)
            .on_input(TunnelsPageMsg::BindPortChanged)
            .padding([9, 10])
            .size(13)
            .font(FONT_MONO)
            .width(Length::FillPortion(2))
            .style(dark_input),
        text("->").size(13).font(FONT_MONO_BOLD).color(TEXT_MUTED),
        text_input(host_placeholder, &st.target_host)
            .on_input(TunnelsPageMsg::TargetHostChanged)
            .padding([9, 10])
            .size(13)
            .font(FONT_MONO)
            .width(Length::FillPortion(2))
            .style(dark_input),
        text(":").size(13).font(FONT_MONO_BOLD).color(TEXT_MUTED),
        text_input(target_placeholder, &st.target_port)
            .on_input(TunnelsPageMsg::TargetPortChanged)
            .on_submit(TunnelsPageMsg::BindSubmit)
            .padding([9, 10])
            .size(13)
            .font(FONT_MONO)
            .width(Length::FillPortion(2))
            .style(dark_input),
        pick_list(protocols, Some(st.protocol.clone()), TunnelsPageMsg::ProtocolSelected)
            .padding([8, 12])
            .text_size(12)
            .width(Length::Fixed(86.0))
            .style(dark_pick_list),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    let bind_btn = button(
        row![
            icons::plug(14.0, Color::WHITE),
            text(if st.busy { "Binding..." } else { "Bind Tunnel" })
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(Color::WHITE),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding([9, 14])
    .style(brand_button);
    let bind_btn = if st.busy {
        bind_btn
    } else {
        bind_btn.on_press(TunnelsPageMsg::BindSubmit)
    };
    form_row = form_row.push(bind_btn);

    let form_card = container(column![dir_pills, form_row].spacing(12))
        .width(Length::Fill)
        .padding(16)
        .style(glass_panel);

    let mut body = column![header, form_card].spacing(16).padding(24);

    if let Some(err) = &st.error {
        body = body.push(
            container(text(err).size(12).color(RED_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
        );
    }

    let table_header = row![
        text("Local / Bind Endpoint")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::FillPortion(3)),
        text("Destination")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::FillPortion(3)),
        text("Type")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::Fixed(100.0)),
        text("Status")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::Fixed(90.0)),
        text("Action")
            .size(11)
            .font(FONT_MONO_BOLD)
            .color(TEXT_MUTED)
            .width(Length::Fixed(64.0)),
    ]
    .padding([8, 12]);

    let mut rows_col = column![table_header].spacing(6);

    if st.tunnels.is_empty() {
        rows_col = rows_col.push(
            container(
                column![
                    icons::network(34.0, TEXT_MUTED),
                    text("No active SSH tunnels on this session.")
                        .size(13)
                        .color(TEXT_SECONDARY),
                ]
                .spacing(8)
                .align_x(alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(32)
            .align_x(alignment::Horizontal::Center),
        );
    } else {
        for tun in &st.tunnels {
            let tid = tun.id.clone();
            let is_https = *st.schemes.get(&tid).unwrap_or(&false);
            let scheme = if is_https { "https" } else { "http" };

            let endpoint_cell: Element<'a, TunnelsPageMsg> = match tun.direction {
                TunnelDirection::Local => {
                    let url = format!("{scheme}://127.0.0.1:{}", tun.local_port);
                    row![
                        button(
                            text(scheme)
                                .size(10)
                                .font(FONT_MONO_BOLD)
                                .color(BRAND_300),
                        )
                        .padding([3, 7])
                        .style(secondary_button)
                        .on_press(TunnelsPageMsg::ToggleScheme(tid.clone())),
                        button(
                            row![
                                text(format!("localhost:{}", tun.local_port))
                                    .size(12)
                                    .font(FONT_MONO_BOLD)
                                    .color(BLUE_400),
                                icons::external_link(12.0, BLUE_400),
                            ]
                            .spacing(5)
                            .align_y(alignment::Vertical::Center),
                        )
                        .padding([4, 6])
                        .style(|_, _| button::Style {
                            background: None,
                            ..Default::default()
                        })
                        .on_press(TunnelsPageMsg::OpenUrl(url)),
                    ]
                    .spacing(6)
                    .align_y(alignment::Vertical::Center)
                    .width(Length::FillPortion(3))
                    .into()
                }
                TunnelDirection::Remote => text(format!("remote:{}", tun.remote_port))
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(BRAND_300)
                    .width(Length::FillPortion(3))
                    .into(),
            };

            let dest_label = match tun.direction {
                TunnelDirection::Local => format!("{}:{}", tun.remote_host, tun.remote_port),
                TunnelDirection::Remote => format!("{}:{}", tun.remote_host, tun.local_port),
            };
            let dir_badge = match tun.direction {
                TunnelDirection::Local => format!("-L {}", tun.protocol),
                TunnelDirection::Remote => format!("-R {}", tun.protocol),
            };

            let tun_row = container(
                row![
                    endpoint_cell,
                    text(dest_label)
                        .size(12)
                        .font(FONT_MONO)
                        .color(TEXT_LIGHT)
                        .width(Length::FillPortion(3)),
                    container(
                        container(
                            text(dir_badge)
                                .size(11)
                                .font(FONT_MONO_BOLD)
                                .color(BRAND_300),
                        )
                        .padding([3, 8])
                        .style(pill_badge(
                            rgba(BRAND_500, 0.14),
                            rgba(BRAND_500, 0.3),
                            BRAND_300,
                        )),
                    )
                    .width(Length::Fixed(100.0)),
                    container(
                        container(
                            text("Active")
                                .size(11)
                                .font(FONT_MONO_BOLD)
                                .color(GREEN_400),
                        )
                        .padding([3, 8])
                        .style(pill_badge(
                            rgba(GREEN_500, 0.14),
                            rgba(GREEN_500, 0.35),
                            GREEN_400,
                        )),
                    )
                    .width(Length::Fixed(90.0)),
                    container(
                        button(icons::trash(14.0, RED_400))
                            .padding([6, 10])
                            .style(danger_ghost_button)
                            .on_press(TunnelsPageMsg::StopTunnel(tid)),
                    )
                    .width(Length::Fixed(64.0)),
                ]
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([10, 12])
            .style(|_| container::Style {
                background: Some(Background::Color(DARK_900)),
                border: Border {
                    color: DARK_700,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            });

            rows_col = rows_col.push(tun_row);
        }
    }

    let table_card = container(
        column![
            row![
                text("Active Tunnels")
                    .size(14)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                space().width(Length::Fill),
                text(format!("{} bound", st.tunnels.len()))
                    .size(11)
                    .font(FONT_MONO)
                    .color(TEXT_MUTED),
            ]
            .align_y(alignment::Vertical::Center),
            scrollable(rows_col)
                .height(Length::Fill)
                .style(dark_scrollable),
        ]
        .spacing(10),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(16)
    .style(|_| container::Style {
        background: Some(Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    });

    body = body.push(table_card);
    body.width(Length::Fill).height(Length::Fill).into()
}
