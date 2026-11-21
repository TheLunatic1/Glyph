use crate::theme::{
    dark_card, dark_scrollable, ghost_icon_button, glass_panel, modal_backdrop, modal_card, rgba,
    server_card_button, truncate_ellipsis, BLUE_400, BRAND_300, BRAND_400, BRAND_500, CYAN_400,
    DARK_600, DARK_700, DARK_800, DARK_900, FONT_MONO, FONT_MONO_BOLD, FONT_MONO_MEDIUM,
    FONT_UI_BOLD, FONT_UI_SEMIBOLD, GREEN_400, ORANGE_400, PURPLE_400, RED_400, RED_500,
    TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400, YELLOW_500,
};
use crate::ui::icons;
use crate::ui::os_logo::os_logo;
use glyph_core::models::{ServerSummary, TelemetrySnapshot};
use glyph_core::telemetry::{fmt_bytes, fmt_speed, is_virtual_interface};
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, Path, Program, Stroke};
use iced::widget::{button, column, container, row, scrollable, space, stack, text};
use iced::{alignment, Border, Color, Element, Length, Point, Radians, Rectangle, Renderer, Theme};
use std::f32::consts::FRAC_PI_2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailModal {
    Cpu,
    Memory,
    Disk,
    Gpu,
    Network,
}

#[derive(Debug, Clone)]
pub enum DashboardMsg {
    OpenModal(DetailModal),
    CloseModal,
}

#[derive(Default)]
pub struct DashboardState {
    pub stats: TelemetrySnapshot,
    pub active_modal: Option<DetailModal>,
}

pub fn view<'a>(
    state: &'a DashboardState,
    server: &'a ServerSummary,
    window_width: f32,
) -> Element<'a, DashboardMsg> {
    let header_left = column![
        text("Server Dashboard")
            .size(28)
            .font(FONT_UI_BOLD)
            .color(TEXT_PRIMARY),
        text("Live metrics - click any card for details")
            .size(13)
            .color(TEXT_SECONDARY),
    ]
    .spacing(6);

    let srv_title = truncate_ellipsis(
        if server.name.is_empty() {
            &server.host
        } else {
            &server.name
        },
        24,
    );
    let srv_sub = truncate_ellipsis(&format!("{}@{}:{}", server.username, server.host, server.port), 30);

    let server_pill = container(
        row![
            container(os_logo(server.os.as_deref(), &server.name, 30.0))
                .width(Length::Fixed(46.0))
                .height(Length::Fixed(46.0))
                .center_x(Length::Fixed(46.0))
                .center_y(Length::Fixed(46.0))
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(DARK_900)),
                    border: Border {
                        color: rgba(BRAND_500, 0.25),
                        width: 1.0,
                        radius: 999.0.into(),
                    },
                    snap: true,
                    ..Default::default()
                }),
            column![
                text(srv_title)
                    .size(17)
                    .font(FONT_UI_BOLD)
                    .color(TEXT_PRIMARY)
                    .wrapping(text::Wrapping::None),
                text(srv_sub)
                    .size(12)
                    .font(FONT_MONO_MEDIUM)
                    .color(TEXT_SECONDARY)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(2),
        ]
        .spacing(14)
        .align_y(alignment::Vertical::Center),
    )
    .max_width(380)
    .clip(true)
    .padding(14)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 14.0.into(),
        },
        snap: true,
        ..Default::default()
    });

    let header = row![header_left, space().width(Length::Fill), server_pill]
        .spacing(16)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill);

    let mut body = column![header].spacing(24).width(Length::Fill);

    if state.stats.cores.len() > 16 {
        let banner = container(
            row![
                icons::info(20.0, BRAND_300),
                column![
                    text("High-capacity host detected (Shared Hosting / Large VM)")
                        .size(13)
                        .font(FONT_UI_BOLD)
                        .color(BRAND_400),
                    text("Your own container stats may not be available. Displayed metrics (like CPU cores and disk) reflect the entire physical server hardware, not just your specific allocation.")
                        .size(12)
                        .color(BRAND_300),
                ]
                .spacing(4),
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Top),
        )
        .padding(16)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(rgba(BRAND_500, 0.10))),
            border: Border {
                color: rgba(BRAND_500, 0.32),
                width: 1.0,
                radius: 12.0.into(),
            },
            snap: true,
            ..Default::default()
        });
        body = body.push(banner);
    }

    // Top circular gauge cards + network card — wrap cleanly onto 2 rows when narrow or 5 cards
    let has_gpu = state.stats.gpu_pct.is_some();
    if !has_gpu && window_width >= 1160.0 {
        let top_cards = row![
            circular_card(state.stats.cpu_pct, "CPU USAGE", DetailModal::Cpu),
            circular_card(state.stats.mem_pct, "MEMORY", DetailModal::Memory),
            circular_card(state.stats.disk_pct, "DISK USAGE", DetailModal::Disk),
            network_card(&state.stats),
        ]
        .spacing(16)
        .width(Length::Fill);
        body = body.push(top_cards);
    } else {
        let row1 = row![
            circular_card(state.stats.cpu_pct, "CPU USAGE", DetailModal::Cpu),
            circular_card(state.stats.mem_pct, "MEMORY", DetailModal::Memory),
            circular_card(state.stats.disk_pct, "DISK USAGE", DetailModal::Disk),
        ]
        .spacing(16)
        .width(Length::Fill);
        let mut row2 = row![].spacing(16).width(Length::Fill);
        if let Some(gpu) = state.stats.gpu_pct {
            row2 = row2.push(circular_card(gpu, "GPU USAGE", DetailModal::Gpu));
        }
        row2 = row2.push(network_card(&state.stats));
        if !has_gpu {
            row2 = row2
                .push(space().width(Length::FillPortion(1)))
                .push(space().width(Length::FillPortion(1)));
        } else {
            row2 = row2.push(space().width(Length::FillPortion(1)));
        }
        body = body.push(row1).push(row2);
    }

    // Bottom 2-column Uptime & Active Users cards
    let uptime_card = container(
        column![
            text("SYSTEM UPTIME").size(13).color(TEXT_SECONDARY),
            text(&state.stats.uptime)
                .size(22)
                .font(FONT_MONO_BOLD)
                .color(BRAND_400),
        ]
        .spacing(10),
    )
    .padding(24)
    .width(Length::FillPortion(1))
    .height(Length::Fixed(136.0))
    .style(glass_panel);

    let users_card = container(
        column![
            text("ACTIVE USERS").size(13).color(TEXT_SECONDARY),
            text(&state.stats.active_users)
                .size(24)
                .font(FONT_MONO_BOLD)
                .color(BRAND_400),
        ]
        .spacing(10),
    )
    .padding(24)
    .width(Length::FillPortion(1))
    .height(Length::Fixed(136.0))
    .style(glass_panel);

    body = body.push(row![uptime_card, users_card].spacing(24).width(Length::Fill));

    let main_scroll: Element<'a, DashboardMsg> = scrollable(container(body).padding(32).width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(dark_scrollable)
        .into();

    if let Some(modal) = state.active_modal {
        let overlay = render_modal(modal, &state.stats);
        stack![main_scroll, overlay].into()
    } else {
        main_scroll
    }
}

fn circular_card<'a>(
    percentage: f32,
    label: &'static str,
    target: DetailModal,
) -> Element<'a, DashboardMsg> {
    let is_na = percentage < 0.0;
    let ring = iced::widget::canvas(RingGauge { percentage })
        .width(Length::Fixed(80.0))
        .height(Length::Fixed(80.0));

    let content = column![
        ring,
        text(label)
            .size(12)
            .color(if is_na { TEXT_MUTED } else { TEXT_SECONDARY }),
    ]
    .spacing(12)
    .align_x(alignment::Horizontal::Center)
    .width(Length::Fill);

    button(container(content).padding([20, 8]).width(Length::Fill))
        .width(Length::FillPortion(1))
        .padding(0)
        .style(server_card_button)
        .on_press(DashboardMsg::OpenModal(target))
        .into()
}

fn network_card<'a>(stats: &TelemetrySnapshot) -> Element<'a, DashboardMsg> {
    let speeds = column![
        row![
            icons::arrow_down(14.0, GREEN_400),
            text(fmt_speed(stats.total_rx_bps))
                .size(16)
                .font(FONT_MONO_BOLD)
                .color(GREEN_400),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
        row![
            icons::arrow_up(14.0, BLUE_400),
            text(fmt_speed(stats.total_tx_bps))
                .size(16)
                .font(FONT_MONO_BOLD)
                .color(BLUE_400),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    ]
    .spacing(8)
    .align_x(alignment::Horizontal::Center)
    .height(Length::Fixed(80.0));

    let content = column![
        container(speeds).center_y(Length::Fixed(80.0)),
        text("NETWORK").size(12).color(TEXT_SECONDARY),
    ]
    .spacing(12)
    .align_x(alignment::Horizontal::Center)
    .width(Length::Fill);

    button(container(content).padding([20, 8]).width(Length::Fill))
        .width(Length::FillPortion(1))
        .padding(0)
        .style(server_card_button)
        .on_press(DashboardMsg::OpenModal(DetailModal::Network))
        .into()
}

struct RingGauge {
    percentage: f32,
}

impl<Message> Program<Message> for RingGauge {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width * 0.5, bounds.height * 0.5);
        let radius = 28.0_f32;
        let is_na = self.percentage < 0.0;
        let pct = if is_na {
            0.0
        } else {
            self.percentage.clamp(0.0, 100.0)
        };

        // Track circle
        frame.stroke(
            &Path::circle(center, radius),
            Stroke {
                style: canvas::Style::Solid(DARK_700),
                width: 6.0,
                line_cap: LineCap::Round,
                ..Default::default()
            },
        );

        if pct > 0.2 {
            let arc_color = if pct > 90.0 {
                RED_500
            } else if pct > 70.0 {
                YELLOW_500
            } else {
                BRAND_500
            };
            let start = -FRAC_PI_2;
            let sweep = (pct / 100.0) * std::f32::consts::TAU;
            let arc_path = Path::new(|b| {
                b.arc(canvas::path::Arc {
                    center,
                    radius,
                    start_angle: Radians(start),
                    end_angle: Radians(start + sweep),
                });
            });
            frame.stroke(
                &arc_path,
                Stroke {
                    style: canvas::Style::Solid(arc_color),
                    width: 6.0,
                    line_cap: LineCap::Round,
                    ..Default::default()
                },
            );
        }

        let label = if is_na {
            "N/A".to_string()
        } else {
            format!("{}%", pct.round() as u32)
        };
        frame.fill_text(canvas::Text {
            content: label,
            position: center,
            color: if is_na { TEXT_MUTED } else { TEXT_PRIMARY },
            size: iced::Pixels(16.0),
            font: FONT_MONO_BOLD,
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: alignment::Vertical::Center,
            ..Default::default()
        });

        vec![frame.into_geometry()]
    }
}

fn render_modal<'a>(modal: DetailModal, stats: &'a TelemetrySnapshot) -> Element<'a, DashboardMsg> {
    let (title, icon, body): (&str, Element<'a, DashboardMsg>, Element<'a, DashboardMsg>) =
        match modal {
            DetailModal::Cpu => (
                "CPU Details",
                icons::cpu(20.0, BRAND_400).into(),
                cpu_modal_body(stats),
            ),
            DetailModal::Memory => (
                "Memory Details",
                icons::memory_stick(20.0, PURPLE_400).into(),
                mem_modal_body(stats),
            ),
            DetailModal::Disk => (
                "Disk Usage",
                icons::hard_drive(20.0, ORANGE_400).into(),
                disk_modal_body(stats),
            ),
            DetailModal::Gpu => (
                "GPU Details",
                icons::circuit_board(20.0, GREEN_400).into(),
                gpu_modal_body(stats),
            ),
            DetailModal::Network => (
                "Network Interfaces",
                icons::wifi(20.0, CYAN_400).into(),
                net_modal_body(stats),
            ),
        };

    let header = container(
        row![
            icon,
            text(title)
                .size(18)
                .font(FONT_UI_BOLD)
                .color(TEXT_PRIMARY),
            space().width(Length::Fill),
            button(icons::x_close(18.0, TEXT_SECONDARY))
                .padding(6)
                .style(ghost_icon_button)
                .on_press(DashboardMsg::CloseModal),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
    )
    .padding(16)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(rgba(DARK_800, 0.55))),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 0.0.into(),
        },
        snap: true,
        ..Default::default()
    });

    let card = container(
        column![
            header,
            scrollable(container(body).padding(24).width(Length::Fill))
                .height(Length::Shrink)
                .style(dark_scrollable),
        ],
    )
    .width(Length::Fixed(680.0))
    .max_height(620.0)
    .style(modal_card);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(20)
        .style(modal_backdrop)
        .into()
}

fn cpu_modal_body<'a>(stats: &'a TelemetrySnapshot) -> Element<'a, DashboardMsg> {
    let mut col = column![].spacing(18).width(Length::Fill);

    if let Some(load) = &stats.load_avg {
        let mut cards = row![].spacing(12).width(Length::Fill);
        for (idx, lbl) in ["1 min", "5 min", "15 min"].iter().enumerate() {
            cards = cards.push(
                container(
                    column![
                        text(&load[idx])
                            .size(22)
                            .font(FONT_MONO_BOLD)
                            .color(BRAND_400),
                        text(*lbl).size(11).color(TEXT_MUTED),
                    ]
                    .spacing(4)
                    .align_x(alignment::Horizontal::Center),
                )
                .padding(12)
                .width(Length::FillPortion(1))
                .center_x(Length::FillPortion(1))
                .style(dark_card),
            );
        }
        col = col.push(
            column![
                text("LOAD AVERAGE").size(11).color(TEXT_SECONDARY),
                cards,
            ]
            .spacing(8),
        );
    }

    if !stats.tasks_line.is_empty() {
        col = col.push(
            container(text(&stats.tasks_line).size(12).font(FONT_MONO).color(TEXT_LIGHT))
                .padding(12)
                .width(Length::Fill)
                .style(dark_card),
        );
    }

    if !stats.cores.is_empty() {
        let mut grid = column![].spacing(8);
        for pair in stats.cores.chunks(2) {
            let mut r = row![].spacing(8).width(Length::Fill);
            for core in pair {
                let col_bar = if core.pct > 90.0 {
                    RED_500
                } else if core.pct > 60.0 {
                    YELLOW_500
                } else {
                    BRAND_500
                };
                let pct_color = if core.pct > 90.0 {
                    RED_400
                } else if core.pct > 60.0 {
                    YELLOW_400
                } else {
                    BRAND_400
                };
                r = r.push(
                    container(
                        column![
                            row![
                                text(&core.id).size(12).font(FONT_MONO).color(TEXT_SECONDARY),
                                space().width(Length::Fill),
                                text(format!("{:.1}%", core.pct))
                                    .size(12)
                                    .font(FONT_MONO_BOLD)
                                    .color(pct_color),
                            ],
                            progress_bar(core.pct, 100.0, col_bar, 6.0),
                        ]
                        .spacing(6),
                    )
                    .padding([8, 12])
                    .width(Length::FillPortion(1))
                    .style(dark_card),
                );
            }
            if pair.len() == 1 {
                r = r.push(space().width(Length::FillPortion(1)));
            }
            grid = grid.push(r);
        }
        col = col.push(
            column![
                text(format!("PER-THREAD USAGE ({} threads)", stats.cores.len()))
                    .size(11)
                    .color(TEXT_SECONDARY),
                grid,
            ]
            .spacing(8),
        );
    }

    if !stats.temps.is_empty() {
        let mut grid = column![].spacing(8);
        for pair in stats.temps.chunks(2) {
            let mut r = row![].spacing(8).width(Length::Fill);
            for t in pair {
                let tc = if t.celsius > 80.0 {
                    RED_400
                } else if t.celsius > 65.0 {
                    YELLOW_400
                } else {
                    GREEN_400
                };
                r = r.push(
                    container(
                        column![
                            text(&t.label).size(12).color(TEXT_SECONDARY),
                            text(if t.celsius > 0.0 {
                                format!("{:.0}°C", t.celsius)
                            } else {
                                "—".to_string()
                            })
                            .size(15)
                            .font(FONT_MONO_BOLD)
                            .color(tc),
                        ]
                        .spacing(4),
                    )
                    .padding([8, 12])
                    .width(Length::FillPortion(1))
                    .style(dark_card),
                );
            }
            if pair.len() == 1 {
                r = r.push(space().width(Length::FillPortion(1)));
            }
            grid = grid.push(r);
        }
        col = col.push(
            column![text("TEMPERATURE").size(11).color(TEXT_SECONDARY), grid].spacing(8),
        );
    } else {
        col = col.push(
            column![
                text("TEMPERATURE").size(11).color(TEXT_SECONDARY),
                container(
                    column![
                        text("No thermal sensors detected. If this is a physical server, install lm-sensors:")
                            .size(13)
                            .color(TEXT_SECONDARY),
                        container(text("sudo apt install lm-sensors").size(13).font(FONT_MONO).color(BRAND_300))
                            .padding(8)
                            .width(Length::Fill)
                            .style(dark_card),
                        text("Note: Virtual Machines (VPS) rarely expose physical thermal sensors.")
                            .size(11)
                            .color(TEXT_MUTED),
                    ]
                    .spacing(8),
                )
                .padding(14)
                .width(Length::Fill)
                .style(dark_card),
            ]
            .spacing(8),
        );
    }

    col.into()
}

fn mem_modal_body<'a>(stats: &'a TelemetrySnapshot) -> Element<'a, DashboardMsg> {
    let m = &stats.mem;
    if m.total_mb <= 0.0 {
        return container(text("Waiting for data...").size(14).color(TEXT_MUTED))
            .center_x(Length::Fill)
            .padding(24)
            .into();
    }
    let human_mb = |v: f32| {
        if v >= 1024.0 {
            format!("{:.1} GB", v / 1024.0)
        } else {
            format!("{:.0} MB", v)
        }
    };

    let mut summary = row![].spacing(10).width(Length::Fill);
    for (lbl, val) in [
        ("Total", m.total_mb),
        ("Used", m.used_mb),
        ("Buff/Cache", m.buff_cache_mb),
        ("Free", m.free_mb),
    ] {
        summary = summary.push(
            container(
                column![
                    text(human_mb(val))
                        .size(18)
                        .font(FONT_MONO_BOLD)
                        .color(TEXT_PRIMARY),
                    text(lbl).size(11).color(TEXT_MUTED),
                ]
                .spacing(4)
                .align_x(alignment::Horizontal::Center),
            )
            .padding(12)
            .width(Length::FillPortion(1))
            .center_x(Length::FillPortion(1))
            .style(dark_card),
        );
    }

    let mut col = column![
        summary,
        text("RAM BREAKDOWN").size(11).color(TEXT_SECONDARY),
        metric_bar("Used", m.used_mb, m.total_mb, " MB", false),
        metric_bar("Buff/Cache", m.buff_cache_mb, m.total_mb, " MB", false),
        metric_bar("Free", m.free_mb, m.total_mb, " MB", true),
    ]
    .spacing(12)
    .width(Length::Fill);

    if m.swap_total_mb > 0.0 {
        col = col
            .push(text("SWAP").size(11).color(TEXT_SECONDARY))
            .push(metric_bar("Swap Used", m.swap_used_mb, m.swap_total_mb, " MB", false))
            .push(metric_bar("Swap Free", m.swap_free_mb, m.swap_total_mb, " MB", true));
    }

    col.into()
}

fn disk_modal_body<'a>(stats: &'a TelemetrySnapshot) -> Element<'a, DashboardMsg> {
    if stats.disks.is_empty() {
        return container(text("Waiting for data...").size(14).color(TEXT_MUTED))
            .center_x(Length::Fill)
            .padding(24)
            .into();
    }

    let mut col = column![].spacing(12).width(Length::Fill);
    for d in &stats.disks {
        let bar_col = if d.pct > 90.0 {
            RED_500
        } else if d.pct > 75.0 {
            YELLOW_500
        } else {
            BRAND_500
        };
        let pct_col = if d.pct > 90.0 {
            RED_400
        } else if d.pct > 75.0 {
            YELLOW_400
        } else {
            BRAND_400
        };
        col = col.push(
            container(
                column![
                    row![
                        text(&d.mountpoint)
                            .size(14)
                            .font(FONT_MONO_BOLD)
                            .color(TEXT_LIGHT),
                        space().width(Length::Fill),
                        text(&d.source).size(12).color(TEXT_MUTED),
                    ],
                    progress_bar(d.pct, 100.0, bar_col, 10.0),
                    row![
                        text(format!("Used: {}", d.used)).size(12).color(TEXT_SECONDARY),
                        space().width(Length::Fill),
                        text(format!("Free: {}", d.avail)).size(12).color(TEXT_SECONDARY),
                        space().width(Length::Fill),
                        text(format!("Total: {}", d.size)).size(12).color(TEXT_SECONDARY),
                        space().width(Length::Fill),
                        text(format!("{:.0}%", d.pct))
                            .size(12)
                            .font(FONT_MONO_BOLD)
                            .color(pct_col),
                    ],
                ]
                .spacing(8),
            )
            .padding(14)
            .width(Length::Fill)
            .style(dark_card),
        );
    }
    col.into()
}

fn gpu_modal_body<'a>(stats: &'a TelemetrySnapshot) -> Element<'a, DashboardMsg> {
    if stats.no_gpu_detected || stats.gpus.is_empty() {
        return container(
            column![
                text("No GPU Stats Available")
                    .size(16)
                    .font(FONT_UI_BOLD)
                    .color(TEXT_PRIMARY),
                text("We could not detect any GPU monitoring tools on this server. Install the matching driver/tools:")
                    .size(13)
                    .color(TEXT_SECONDARY),
                container(
                    column![
                        text("NVIDIA GPUs").size(13).font(FONT_UI_SEMIBOLD).color(GREEN_400),
                        text("sudo apt install nvidia-utils-535").size(13).font(FONT_MONO).color(BRAND_300),
                    ]
                    .spacing(4),
                )
                .padding(12)
                .width(Length::Fill)
                .style(dark_card),
                container(
                    column![
                        text("AMD GPUs").size(13).font(FONT_MONO_BOLD).color(RED_400),
                        text("sudo apt install rocm-smi").size(13).font(FONT_MONO).color(BRAND_300),
                    ]
                    .spacing(4),
                )
                .padding(12)
                .width(Length::Fill)
                .style(dark_card),
                container(
                    column![
                        text("Intel GPUs").size(13).font(FONT_MONO_BOLD).color(BLUE_400),
                        text("sudo apt install intel-gpu-tools").size(13).font(FONT_MONO).color(BRAND_300),
                    ]
                    .spacing(4),
                )
                .padding(12)
                .width(Length::Fill)
                .style(dark_card),
            ]
            .spacing(12),
        )
        .padding(16)
        .width(Length::Fill)
        .style(dark_card)
        .into();
    }

    let mut col = column![].spacing(14).width(Length::Fill);
    for g in &stats.gpus {
        let tc = if g.temp_c > 80.0 {
            RED_400
        } else if g.temp_c > 65.0 {
            YELLOW_400
        } else {
            GREEN_400
        };
        let uc = if g.util_pct > 90.0 {
            RED_400
        } else if g.util_pct > 70.0 {
            YELLOW_400
        } else {
            BRAND_400
        };
        col = col.push(
            container(
                column![
                    row![
                        text(&g.name)
                            .size(16)
                            .font(FONT_MONO_BOLD)
                            .color(TEXT_PRIMARY),
                        space().width(Length::Fill),
                        text(format!("GPU {}", g.index))
                            .size(13)
                            .font(FONT_MONO)
                            .color(TEXT_MUTED),
                    ],
                    row![
                        container(
                            column![
                                text("TEMPERATURE").size(11).color(TEXT_SECONDARY),
                                text(if g.temp_c > 0.0 {
                                    format!("{:.0}°C", g.temp_c)
                                } else {
                                    "—".to_string()
                                })
                                .size(18)
                                .font(FONT_MONO_BOLD)
                                .color(tc),
                            ]
                            .spacing(4),
                        )
                        .padding(12)
                        .width(Length::FillPortion(1))
                        .style(glass_panel),
                        container(
                            column![
                                text("CORE USAGE").size(11).color(TEXT_SECONDARY),
                                text(format!("{:.0}%", g.util_pct))
                                    .size(18)
                                    .font(FONT_MONO_BOLD)
                                    .color(uc),
                            ]
                            .spacing(4),
                        )
                        .padding(12)
                        .width(Length::FillPortion(1))
                        .style(glass_panel),
                    ]
                    .spacing(12),
                    metric_bar("VRAM Usage", g.mem_used_mb, g.mem_total_mb, " MB", false),
                ]
                .spacing(12),
            )
            .padding(16)
            .width(Length::Fill)
            .style(dark_card),
        );
    }
    col.into()
}

fn net_modal_body<'a>(stats: &'a TelemetrySnapshot) -> Element<'a, DashboardMsg> {
    if stats.net_interfaces.is_empty() {
        return container(text("Waiting for network data...").size(14).color(TEXT_MUTED))
            .center_x(Length::Fill)
            .padding(24)
            .into();
    }

    let hdr = row![
        text("INTERFACE").size(11).color(TEXT_MUTED).width(Length::FillPortion(2)),
        text("↓ SPEED").size(11).color(GREEN_400).width(Length::FillPortion(2)),
        text("↑ SPEED").size(11).color(BLUE_400).width(Length::FillPortion(2)),
        text("TOTAL RX").size(11).color(TEXT_MUTED).width(Length::FillPortion(2)),
        text("TOTAL TX").size(11).color(TEXT_MUTED).width(Length::FillPortion(2)),
    ]
    .padding([0, 12]);

    let mut col = column![hdr].spacing(8).width(Length::Fill);
    for iface in &stats.net_interfaces {
        let dot_color = if is_virtual_interface(&iface.iface) {
            DARK_600
        } else {
            GREEN_400
        };
        col = col.push(
            container(
                row![
                    row![
                        container(text(""))
                            .width(Length::Fixed(8.0))
                            .height(Length::Fixed(8.0))
                            .style(move |_| container::Style {
                                background: Some(iced::Background::Color(dot_color)),
                                border: Border {
                                    radius: 999.0.into(),
                                    ..Default::default()
                                },
                                snap: true,
                                ..Default::default()
                            }),
                        text(&iface.iface).size(13).font(FONT_MONO_BOLD).color(TEXT_PRIMARY),
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center)
                    .width(Length::FillPortion(2)),
                    text(fmt_speed(iface.rx_bps))
                        .size(13)
                        .font(FONT_MONO_BOLD)
                        .color(GREEN_400)
                        .width(Length::FillPortion(2)),
                    text(fmt_speed(iface.tx_bps))
                        .size(13)
                        .font(FONT_MONO_BOLD)
                        .color(BLUE_400)
                        .width(Length::FillPortion(2)),
                    text(fmt_bytes(iface.rx_bytes))
                        .size(12)
                        .font(FONT_MONO)
                        .color(TEXT_SECONDARY)
                        .width(Length::FillPortion(2)),
                    text(fmt_bytes(iface.tx_bytes))
                        .size(12)
                        .font(FONT_MONO)
                        .color(TEXT_SECONDARY)
                        .width(Length::FillPortion(2)),
                ]
                .align_y(alignment::Vertical::Center),
            )
            .padding([12, 12])
            .width(Length::Fill)
            .style(dark_card),
        );
    }
    col = col.push(
        container(text("Speed updates every 3 seconds").size(11).color(TEXT_MUTED))
            .center_x(Length::Fill)
            .padding([8, 0]),
    );
    col.into()
}

fn metric_bar<'a>(
    label: &'a str,
    value: f32,
    max: f32,
    unit: &'a str,
    inverted: bool,
) -> Element<'a, DashboardMsg> {
    let pct = if max > 0.0 {
        ((value / max) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let score = if inverted { 100.0 - pct } else { pct };
    let color = if score > 90.0 {
        RED_500
    } else if score > 75.0 {
        YELLOW_500
    } else {
        BRAND_500
    };

    column![
        row![
            text(label).size(12).color(TEXT_SECONDARY),
            space().width(Length::Fill),
            text(format!("{:.0}{unit} / {:.0}{unit}", value, max))
                .size(12)
                .font(FONT_MONO)
                .color(TEXT_LIGHT),
        ],
        progress_bar(value, max, color, 8.0),
    ]
    .spacing(4)
    .into()
}

fn progress_bar<'a>(
    value: f32,
    max: f32,
    color: Color,
    height: f32,
) -> Element<'a, DashboardMsg> {
    let pct = if max > 0.0 {
        (value / max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let fill_portion = ((pct * 1000.0).round() as u16).max(1);
    let rest_portion = 1000u16.saturating_sub(fill_portion);

    let bar_row = if rest_portion == 0 {
        row![container(text(""))
            .width(Length::Fill)
            .height(Length::Fixed(height))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(color)),
                border: Border {
                    radius: 999.0.into(),
                    ..Default::default()
                },
                snap: true,
                ..Default::default()
            })]
    } else if pct <= 0.001 {
        row![space().width(Length::Fill)]
    } else {
        row![
            container(text(""))
                .width(Length::FillPortion(fill_portion))
                .height(Length::Fixed(height))
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(color)),
                    border: Border {
                        radius: 999.0.into(),
                        ..Default::default()
                    },
                    snap: true,
                    ..Default::default()
                }),
            space().width(Length::FillPortion(rest_portion)),
        ]
    };

    container(bar_row)
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(DARK_700)),
            border: Border {
                radius: 999.0.into(),
                ..Default::default()
            },
            snap: true,
            ..Default::default()
        })
        .into()
}
