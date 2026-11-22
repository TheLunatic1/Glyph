use crate::theme::{
    brand_button, dark_card, dark_scrollable, ghost_icon_button, modal_backdrop, modal_card,
    pill_badge, rgba, secondary_button, APP_VERSION, BRAND_300, BRAND_400, BRAND_500, DARK_700,
    DARK_800, DARK_900, FONT_MONO, FONT_MONO_BOLD, FONT_UI, FONT_UI_BOLD, FONT_UI_SEMIBOLD,
    GREEN_400, GREEN_500, RED_400, RED_500, TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons;
use glyph_core::updater::{
    select_desktop_update_asset, DownloadedUpdate, ReleaseInfo, UpdateDownloadProgress,
    UpdateInstallKind,
};
use iced::widget::{button, column, container, mouse_area, row, scrollable, space, stack, text};
use iced::{alignment, Background, Border, Color, Element, Length, Shadow, Theme};

/// Current lifecycle stage of the native in-app desktop updater.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateStage {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Downloaded,
    Installing,
    Error,
}

/// Shared updater state rendered across the Manager banner, Server windows, Settings tab, and modal.
#[derive(Debug, Clone, Default)]
pub struct AppUpdateState {
    pub stage: UpdateStage,
    pub info: Option<ReleaseInfo>,
    pub banner_dismissed: bool,
    pub modal_open: bool,
    pub progress: Option<UpdateDownloadProgress>,
    pub downloaded: Option<DownloadedUpdate>,
    pub error: Option<String>,
}

/// UI actions emitted by the update banner, update modal, and Settings -> Updates controls.
#[derive(Debug, Clone)]
pub enum UpdateMsg {
    OpenModal,
    CloseModal,
    DismissBanner,
    StartDownload,
    CancelDownload,
    InstallNow,
    OpenReleasePage,
    CheckNow,
    Noop,
}

/// Formats a byte count into human-readable units (`B`, `KB`, `MB`, `GB`).
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.2} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}

/// Formats a transfer rate in bytes/sec into `B/s`, `KB/s`, or `MB/s`.
pub fn format_speed(bps: f64) -> String {
    if bps >= 1_000_000.0 {
        format!("{:.1} MB/s", bps / 1_000_000.0)
    } else if bps >= 1_000.0 {
        format!("{:.1} KB/s", bps / 1_000.0)
    } else {
        format!("{:.0} B/s", bps.max(0.0))
    }
}

fn strip_markdown_emphasis(input: &str) -> String {
    input.replace("**", "").replace('`', "")
}

fn render_markdown_lite<'a>(notes: &str) -> Element<'a, UpdateMsg> {
    if notes.trim().is_empty() {
        return text("No release notes published for this version.")
            .size(12)
            .color(TEXT_MUTED)
            .into();
    }

    let mut lines_col = column![].spacing(5).width(Length::Fill);
    for raw_line in notes.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            lines_col = lines_col.push(space().height(4));
            continue;
        }
        if trimmed == "---" || trimmed == "***" {
            lines_col = lines_col.push(
                container(space().width(Length::Fill).height(1)).style(|_| container::Style {
                    background: Some(Background::Color(DARK_700)),
                    ..Default::default()
                }),
            );
            continue;
        }
        if let Some(h) = trimmed.strip_prefix("### ") {
            lines_col = lines_col.push(
                text(strip_markdown_emphasis(h))
                    .size(13)
                    .font(FONT_UI_BOLD)
                    .color(BRAND_300),
            );
        } else if let Some(h) = trimmed.strip_prefix("## ") {
            lines_col = lines_col.push(
                text(strip_markdown_emphasis(h))
                    .size(14)
                    .font(FONT_UI_BOLD)
                    .color(TEXT_PRIMARY),
            );
        } else if let Some(h) = trimmed.strip_prefix("# ") {
            lines_col = lines_col.push(
                text(strip_markdown_emphasis(h))
                    .size(15)
                    .font(FONT_UI_BOLD)
                    .color(TEXT_PRIMARY),
            );
        } else if let Some(quote) = trimmed.strip_prefix("> ") {
            lines_col = lines_col.push(
                container(
                    text(strip_markdown_emphasis(quote))
                        .size(12)
                        .font(FONT_UI)
                        .color(BRAND_300),
                )
                .width(Length::Fill)
                .padding([6, 10])
                .style(|_| container::Style {
                    background: Some(Background::Color(rgba(BRAND_500, 0.10))),
                    border: Border {
                        color: rgba(BRAND_500, 0.35),
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    snap: true,
                    ..Default::default()
                }),
            );
        } else if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            lines_col = lines_col.push(
                row![
                    text("•").size(12).color(BRAND_400),
                    text(strip_markdown_emphasis(item))
                        .size(12)
                        .font(FONT_UI)
                        .color(TEXT_LIGHT),
                ]
                .spacing(7)
                .align_y(alignment::Vertical::Top),
            );
        } else {
            lines_col = lines_col.push(
                text(strip_markdown_emphasis(trimmed))
                    .size(12)
                    .font(FONT_UI)
                    .color(TEXT_SECONDARY),
            );
        }
    }

    lines_col.into()
}

fn banner_pill_button(_: &Theme, status: button::Status) -> button::Style {
    let alpha = match status {
        button::Status::Hovered | button::Status::Pressed => 0.28,
        _ => 0.18,
    };
    button::Style {
        background: Some(Background::Color(rgba(Color::WHITE, alpha))),
        text_color: Color::WHITE,
        border: Border {
            color: rgba(Color::WHITE, 0.25),
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn banner_ghost_button(_: &Theme, status: button::Status) -> button::Style {
    let alpha = match status {
        button::Status::Hovered | button::Status::Pressed => 0.22,
        _ => 0.10,
    };
    button::Style {
        background: Some(Background::Color(rgba(Color::WHITE, alpha))),
        text_color: Color::WHITE,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn emerald_button(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => GREEN_400,
        button::Status::Disabled => rgba(GREEN_500, 0.45),
        button::Status::Active => GREEN_500,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::from_rgb8(6, 20, 12),
        border: Border {
            color: rgba(GREEN_400, 0.45),
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Renders the non-intrusive top update notification strip for Manager and Server windows.
pub fn view_banner<'a>(state: &'a AppUpdateState) -> Option<Element<'a, UpdateMsg>> {
    match state.stage {
        UpdateStage::Available if !state.banner_dismissed => {
            let rel = state.info.as_ref()?;
            let bar = container(
                row![
                    icons::sparkles(15.0, Color::WHITE),
                    text(format!("Glyph {} is available!", rel.version))
                        .size(12)
                        .font(FONT_UI_SEMIBOLD)
                        .color(Color::WHITE),
                    button(
                        row![
                            text("View Release Notes & Update")
                                .size(11)
                                .font(FONT_UI_SEMIBOLD)
                                .color(Color::WHITE),
                            icons::arrow_right(12.0, Color::WHITE),
                        ]
                        .spacing(5)
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding([5, 10])
                    .style(banner_pill_button)
                    .on_press(UpdateMsg::OpenModal),
                    button(
                        row![
                            icons::external_link(12.0, Color::WHITE),
                            text("Download Manually").size(11).color(Color::WHITE),
                        ]
                        .spacing(5)
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding([5, 10])
                    .style(banner_ghost_button)
                    .on_press(UpdateMsg::OpenReleasePage),
                    space().width(Length::Fill),
                    button(icons::x_close(14.0, Color::WHITE))
                        .padding([4, 6])
                        .style(banner_ghost_button)
                        .on_press(UpdateMsg::DismissBanner),
                ]
                .spacing(10)
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([7, 16])
            .style(|_| container::Style {
                background: Some(Background::Color(BRAND_500)),
                snap: true,
                ..Default::default()
            });
            Some(bar.into())
        }
        UpdateStage::Downloading => {
            let ver = state
                .info
                .as_ref()
                .map(|r| r.version.as_str())
                .unwrap_or("update");
            let pct = state.progress.as_ref().map(|p| p.percent).unwrap_or(0);
            let spd = state
                .progress
                .as_ref()
                .filter(|p| p.bytes_per_sec > 0.0)
                .map(|p| format!(" · {}", format_speed(p.bytes_per_sec)))
                .unwrap_or_default();

            let bar = container(
                row![
                    icons::download(14.0, BRAND_300),
                    text(format!("Downloading Glyph {ver}... {pct}%{spd}"))
                        .size(12)
                        .font(FONT_MONO_BOLD)
                        .color(BRAND_300),
                    space().width(Length::Fill),
                    button(
                        text("View Progress")
                            .size(11)
                            .font(FONT_UI_SEMIBOLD)
                            .color(BRAND_300),
                    )
                    .padding([5, 10])
                    .style(secondary_button)
                    .on_press(UpdateMsg::OpenModal),
                ]
                .spacing(10)
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([7, 16])
            .style(|_| container::Style {
                background: Some(Background::Color(DARK_800)),
                border: Border {
                    color: rgba(BRAND_500, 0.35),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                snap: true,
                ..Default::default()
            });
            Some(bar.into())
        }
        UpdateStage::Downloaded => {
            let ver = state
                .downloaded
                .as_ref()
                .map(|d| d.version.as_str())
                .or_else(|| state.info.as_ref().map(|r| r.version.as_str()))
                .unwrap_or("Update");
            let install_label = if state
                .downloaded
                .as_ref()
                .is_some_and(|d| d.install_kind == UpdateInstallKind::MacOsDmgOpen)
            {
                "Open Disk Image"
            } else {
                "Restart & Install"
            };
            let bar = container(
                row![
                    icons::check(15.0, GREEN_400),
                    text(format!("Glyph {ver} is ready to install!"))
                        .size(12)
                        .font(FONT_UI_SEMIBOLD)
                        .color(GREEN_400),
                    space().width(Length::Fill),
                    button(
                        row![
                            icons::rotate_ccw(12.0, Color::from_rgb8(6, 20, 12)),
                            text(install_label)
                                .size(11)
                                .font(FONT_UI_BOLD)
                                .color(Color::from_rgb8(6, 20, 12)),
                        ]
                        .spacing(5)
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding([5, 12])
                    .style(emerald_button)
                    .on_press(UpdateMsg::InstallNow),
                    button(text("View Details").size(11).color(TEXT_LIGHT))
                        .padding([5, 10])
                        .style(secondary_button)
                        .on_press(UpdateMsg::OpenModal),
                ]
                .spacing(10)
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([7, 16])
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb8(13, 40, 29))),
                border: Border {
                    color: rgba(GREEN_500, 0.35),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                snap: true,
                ..Default::default()
            });
            Some(bar.into())
        }
        _ => None,
    }
}

/// Renders the centered update modal dialog overlay with release notes, download progress, and install controls.
pub fn view_modal<'a>(state: &'a AppUpdateState) -> Element<'a, UpdateMsg> {
    let ver_badge = state.info.as_ref().map(|rel| {
        container(
            text(format!("v{APP_VERSION} -> {}", rel.version))
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(BRAND_300),
        )
        .padding([3, 9])
        .style(pill_badge(
            rgba(BRAND_500, 0.16),
            rgba(BRAND_500, 0.35),
            BRAND_300,
        ))
    });

    let title_text = match state.stage {
        UpdateStage::Downloading => "Downloading Update",
        UpdateStage::Downloaded => "Update Ready to Install",
        UpdateStage::Installing => "Installing Update...",
        UpdateStage::Error => "Update Failed",
        _ => "Glyph Software Update",
    };

    let mut header = row![
        container(icons::zap(18.0, BRAND_400))
            .padding(9)
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
            text(title_text)
                .size(17)
                .font(FONT_UI_BOLD)
                .color(TEXT_PRIMARY),
            text("Signed & Verified GitHub Release Channel")
                .size(11)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    if let Some(badge) = ver_badge {
        header = header.push(badge);
    }

    if state.stage != UpdateStage::Installing {
        header = header.push(
            button(icons::x_close(16.0, TEXT_SECONDARY))
                .padding(6)
                .style(ghost_icon_button)
                .on_press(UpdateMsg::CloseModal),
        );
    }

    let body: Element<'a, UpdateMsg> = match state.stage {
        UpdateStage::Available => view_available_body(state),
        UpdateStage::Downloading => view_downloading_body(state),
        UpdateStage::Downloaded => view_downloaded_body(state),
        UpdateStage::Installing => container(
            column![
                icons::rotate_ccw(34.0, BRAND_400),
                text("Applying update and restarting Glyph...")
                    .size(15)
                    .font(FONT_UI_SEMIBOLD)
                    .color(TEXT_PRIMARY),
                text("Your saved hosts, SSH keys, and vault remain untouched.")
                    .size(12)
                    .color(TEXT_SECONDARY),
            ]
            .spacing(10)
            .align_x(alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .padding([28, 16])
        .align_x(alignment::Horizontal::Center)
        .into(),
        UpdateStage::Error => view_error_body(state),
        UpdateStage::Idle | UpdateStage::Checking | UpdateStage::UpToDate => container(
            column![
                text(match state.stage {
                    UpdateStage::Checking => "Checking GitHub Releases for updates...",
                    UpdateStage::UpToDate => "You are running the latest version of Glyph.",
                    _ => "Check GitHub Releases for newer desktop builds.",
                })
                .size(13)
                .color(TEXT_LIGHT),
                row![
                    space().width(Length::Fill),
                    button(text("Close").size(12).color(TEXT_LIGHT))
                        .padding([8, 16])
                        .style(secondary_button)
                        .on_press(UpdateMsg::CloseModal),
                    button(
                        row![
                            icons::refresh_cw(13.0, Color::WHITE),
                            text("Check Now")
                                .size(12)
                                .font(FONT_UI_SEMIBOLD)
                                .color(Color::WHITE),
                        ]
                        .spacing(6)
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding([8, 16])
                    .style(brand_button)
                    .on_press(UpdateMsg::CheckNow),
                ]
                .spacing(10),
            ]
            .spacing(14),
        )
        .width(Length::Fill)
        .into(),
    };

    let dialog = container(column![header, body].spacing(16))
        .width(Length::Fixed(560.0))
        .padding(24)
        .style(modal_card);

    let backdrop_click = if state.stage == UpdateStage::Installing {
        UpdateMsg::Noop
    } else {
        UpdateMsg::CloseModal
    };

    let backdrop = mouse_area(
        container(space().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(modal_backdrop),
    )
    .on_press(backdrop_click);

    let centered_dialog = container(mouse_area(dialog).on_press(UpdateMsg::Noop))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .padding(24);

    stack![backdrop, centered_dialog]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_available_body<'a>(state: &'a AppUpdateState) -> Element<'a, UpdateMsg> {
    let Some(rel) = state.info.as_ref() else {
        return text("No release metadata loaded.").size(12).color(TEXT_MUTED).into();
    };

    let selected = select_desktop_update_asset(rel);
    let mut col = column![].spacing(12).width(Length::Fill);

    if let Some((asset, _)) = &selected {
        let size_label = if asset.size > 0 {
            format!(" · {}", format_bytes(asset.size))
        } else {
            String::new()
        };
        col = col.push(
            container(
                row![
                    icons::download(14.0, BRAND_300),
                    text(format!("Package: {}{}", asset.name, size_label))
                        .size(11)
                        .font(FONT_MONO)
                        .color(BRAND_300),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([8, 12])
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.10))),
                border: Border {
                    color: rgba(BRAND_500, 0.28),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                snap: true,
                ..Default::default()
            }),
        );
    } else {
        col = col.push(
            text("No matching direct binary was found for this OS; use Open GitHub to download manually.")
                .size(12)
                .color(TEXT_SECONDARY),
        );
    }

    let notes_box = container(
        scrollable(render_markdown_lite(&rel.release_notes))
            .height(Length::Fixed(240.0))
            .style(dark_scrollable),
    )
    .width(Length::Fill)
    .padding(14)
    .style(|_| container::Style {
        background: Some(Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 10.0.into(),
        },
        snap: true,
        ..Default::default()
    });
    col = col.push(notes_box);

    let mut actions = row![
        button(text("Later").size(12).color(TEXT_LIGHT))
            .padding([9, 14])
            .style(secondary_button)
            .on_press(UpdateMsg::CloseModal),
        button(
            row![
                icons::external_link(13.0, TEXT_LIGHT),
                text("Open GitHub").size(12).color(TEXT_LIGHT),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([9, 14])
        .style(secondary_button)
        .on_press(UpdateMsg::OpenReleasePage),
        space().width(Length::Fill),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    if selected.is_some() {
        actions = actions.push(
            button(
                row![
                    icons::download(14.0, Color::WHITE),
                    text("Download & Install")
                        .size(12)
                        .font(FONT_UI_BOLD)
                        .color(Color::WHITE),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([9, 18])
            .style(brand_button)
            .on_press(UpdateMsg::StartDownload),
        );
    }

    col.push(actions).into()
}

fn view_downloading_body<'a>(state: &'a AppUpdateState) -> Element<'a, UpdateMsg> {
    let ver = state
        .info
        .as_ref()
        .map(|r| r.version.as_str())
        .unwrap_or("update");
    let prog = state.progress.as_ref();
    let pct = prog.map(|p| p.percent).unwrap_or(0);
    let speed_str = prog
        .filter(|p| p.bytes_per_sec > 0.0)
        .map(|p| format_speed(p.bytes_per_sec))
        .unwrap_or_else(|| "starting...".to_string());
    let transferred = prog.map(|p| format_bytes(p.downloaded_bytes)).unwrap_or_else(|| "0 B".into());
    let total = prog
        .filter(|p| p.total_bytes > 0)
        .map(|p| format_bytes(p.total_bytes))
        .unwrap_or_else(|| "—".into());

    let fill_pct = pct.min(100);
    let track: Element<'a, UpdateMsg> = if fill_pct == 0 {
        container(space().width(Length::Fill).height(12))
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(DARK_700)),
                border: Border {
                    radius: 999.0.into(),
                    ..Default::default()
                },
                snap: true,
                ..Default::default()
            })
            .into()
    } else if fill_pct >= 100 {
        container(space().width(Length::Fill).height(12))
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(BRAND_500)),
                border: Border {
                    radius: 999.0.into(),
                    ..Default::default()
                },
                snap: true,
                ..Default::default()
            })
            .into()
    } else {
        let filled = container(space().width(Length::Fill).height(12))
            .width(Length::FillPortion(u16::from(fill_pct)))
            .style(|_| container::Style {
                background: Some(Background::Color(BRAND_500)),
                border: Border {
                    radius: 999.0.into(),
                    ..Default::default()
                },
                snap: true,
                ..Default::default()
            });
        let remainder = space().width(Length::FillPortion(u16::from(100 - fill_pct)));
        container(row![filled, remainder].width(Length::Fill))
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(DARK_700)),
                border: Border {
                    radius: 999.0.into(),
                    ..Default::default()
                },
                snap: true,
                ..Default::default()
            })
            .into()
    };

    let progress_card = container(
        column![
            row![
                text(format!("Downloading {ver}..."))
                    .size(12)
                    .font(FONT_UI_SEMIBOLD)
                    .color(TEXT_PRIMARY),
                space().width(Length::Fill),
                text(format!("{pct}% · {speed_str}"))
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(BRAND_300),
            ]
            .align_y(alignment::Vertical::Center),
            track,
            text(format!("{transferred} / {total}"))
                .size(11)
                .font(FONT_MONO)
                .color(TEXT_MUTED),
        ]
        .spacing(10),
    )
    .width(Length::Fill)
    .padding(16)
    .style(dark_card);

    column![
        progress_card,
        text("Please keep Glyph open while the release binary downloads and is SHA-256 verified.")
            .size(12)
            .color(TEXT_SECONDARY),
        row![
            button(text("Hide in Background").size(12).color(TEXT_LIGHT))
                .padding([9, 14])
                .style(secondary_button)
                .on_press(UpdateMsg::CloseModal),
            space().width(Length::Fill),
            button(text("Cancel").size(12).color(RED_400))
                .padding([9, 14])
                .style(secondary_button)
                .on_press(UpdateMsg::CancelDownload),
        ]
        .spacing(10),
    ]
    .spacing(14)
    .width(Length::Fill)
    .into()
}

fn view_downloaded_body<'a>(state: &'a AppUpdateState) -> Element<'a, UpdateMsg> {
    let ver = state
        .downloaded
        .as_ref()
        .map(|d| d.version.as_str())
        .or_else(|| state.info.as_ref().map(|r| r.version.as_str()))
        .unwrap_or("Update");
    let asset = state
        .downloaded
        .as_ref()
        .map(|d| d.asset_name.as_str())
        .unwrap_or("Release binary");
    let verified = state.downloaded.as_ref().is_some_and(|d| d.sha256_verified);
    let is_dmg = state
        .downloaded
        .as_ref()
        .is_some_and(|d| d.install_kind == UpdateInstallKind::MacOsDmgOpen);

    let mut content = column![
        container(icons::check(30.0, GREEN_400))
            .padding(14)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(GREEN_500, 0.14))),
                border: Border {
                    color: rgba(GREEN_500, 0.35),
                    width: 1.0,
                    radius: 14.0.into(),
                },
                snap: true,
                ..Default::default()
            }),
        text(format!("Glyph {ver} is ready!"))
            .size(17)
            .font(FONT_UI_BOLD)
            .color(TEXT_PRIMARY),
        text(if is_dmg {
            "The universal macOS disk image is downloaded. Open it to drag Glyph.app to Applications."
        } else {
            "Glyph will close cleanly, replace the application binaries, and relaunch."
        })
        .size(12)
        .color(TEXT_SECONDARY),
        container(
            row![
                icons::shield_check(14.0, if verified { GREEN_400 } else { TEXT_SECONDARY }),
                text(if verified {
                    format!("SHA-256 Verified · {asset}")
                } else {
                    format!("Downloaded · {asset}")
                })
                .size(11)
                .font(FONT_MONO)
                .color(if verified { GREEN_400 } else { TEXT_LIGHT }),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([6, 12])
        .style(pill_badge(
            if verified {
                rgba(GREEN_500, 0.12)
            } else {
                DARK_800
            },
            if verified {
                rgba(GREEN_500, 0.35)
            } else {
                DARK_700
            },
            if verified { GREEN_400 } else { TEXT_LIGHT },
        )),
    ]
    .spacing(10)
    .align_x(alignment::Horizontal::Center);

    let btn_label = if is_dmg {
        "Open Disk Image"
    } else {
        "Restart & Install"
    };

    content = content.push(
        row![
            button(text("Later").size(12).color(TEXT_LIGHT))
                .padding([10, 16])
                .style(secondary_button)
                .on_press(UpdateMsg::CloseModal),
            button(
                row![
                    icons::rotate_ccw(15.0, Color::from_rgb8(6, 20, 12)),
                    text(btn_label)
                        .size(13)
                        .font(FONT_UI_BOLD)
                        .color(Color::from_rgb8(6, 20, 12)),
                ]
                .spacing(7)
                .align_y(alignment::Vertical::Center),
            )
            .padding([10, 22])
            .style(emerald_button)
            .on_press(UpdateMsg::InstallNow),
        ]
        .spacing(10),
    );

    container(content)
        .width(Length::Fill)
        .padding([12, 8])
        .align_x(alignment::Horizontal::Center)
        .into()
}

fn view_error_body<'a>(state: &'a AppUpdateState) -> Element<'a, UpdateMsg> {
    let err_msg = state
        .error
        .as_deref()
        .unwrap_or("An unexpected error occurred while checking or downloading the update.");

    column![
        container(
            row![
                icons::alert_triangle(18.0, RED_400),
                text("Something went wrong during the update process.")
                    .size(13)
                    .font(FONT_UI_SEMIBOLD)
                    .color(TEXT_PRIMARY),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding(12)
        .style(|_| container::Style {
            background: Some(Background::Color(rgba(RED_500, 0.12))),
            border: Border {
                color: rgba(RED_500, 0.35),
                width: 1.0,
                radius: 8.0.into(),
            },
            snap: true,
            ..Default::default()
        }),
        container(
            scrollable(text(err_msg).size(11).font(FONT_MONO).color(RED_400))
                .height(Length::Fixed(110.0))
                .style(dark_scrollable),
        )
        .width(Length::Fill)
        .padding(12)
        .style(|_| container::Style {
            background: Some(Background::Color(DARK_900)),
            border: Border {
                color: DARK_700,
                width: 1.0,
                radius: 8.0.into(),
            },
            snap: true,
            ..Default::default()
        }),
        row![
            button(text("Close").size(12).color(TEXT_LIGHT))
                .padding([9, 14])
                .style(secondary_button)
                .on_press(UpdateMsg::CloseModal),
            space().width(Length::Fill),
            button(
                row![
                    icons::external_link(13.0, TEXT_LIGHT),
                    text("Download Manually").size(12).color(TEXT_LIGHT),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([9, 14])
            .style(secondary_button)
            .on_press(UpdateMsg::OpenReleasePage),
            button(
                row![
                    icons::rotate_ccw(13.0, Color::WHITE),
                    text("Try Again")
                        .size(12)
                        .font(FONT_UI_SEMIBOLD)
                        .color(Color::WHITE),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([9, 16])
            .style(brand_button)
            .on_press(UpdateMsg::StartDownload),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}
