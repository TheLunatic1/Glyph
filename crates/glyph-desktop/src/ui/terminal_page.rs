use crate::theme::{
    brand_button, dark_800_bg, dark_card, dark_input, dark_pick_list, dark_scrollable,
    ghost_icon_button, rgba, secondary_button, BRAND_300, BRAND_400, BRAND_500, DARK_600, DARK_700,
    DARK_800, DARK_900, FONT_MONO, FONT_MONO_BOLD, RED_400, TEXT_LIGHT, TEXT_MUTED, TEXT_SECONDARY,
};
use crate::ui::icons;
use glyph_core::models::{CommandSnippet, SecretSummary};
use glyph_core::ssh::{ShellHandle, SshSession};
use glyph_terminal::{TerminalCanvas, TerminalEmulator, TerminalEvent, TerminalThemeId};
use iced::widget::{
    button, canvas, column, container, pick_list, row, scrollable, space, stack, text, text_input,
};
use iced::{alignment, Border, Element, Length};

#[derive(Debug, Clone)]
pub enum TerminalPageMsg {
    NewTab,
    SelectTab(String),
    CloseTab(String),
    ToggleSplit,
    SelectTheme(TerminalThemeId),
    FontSmaller,
    FontLarger,
    ToggleSearch,
    SearchInput(String),
    SearchNext,
    SearchPrev,
    ClearActive,
    ToggleSnippets,
    ToggleSecrets,
    RunSnippet(String),
    InjectSecret(String),
    CanvasAction(String, TerminalEvent),
}

pub struct TerminalTab {
    pub id: String,
    pub title: String,
    pub emulator: TerminalEmulator,
    pub shell: Option<ShellHandle>,
}

pub struct TerminalPageState {
    pub tabs: Vec<TerminalTab>,
    pub active_tab_id: String,
    pub split_tab_id: Option<String>,
    pub focused_tab_id: String,
    pub font_size: f32,
    pub theme: TerminalThemeId,
    pub show_search: bool,
    pub search_text: String,
    pub show_snippets: bool,
    pub show_secrets: bool,
    tab_counter: usize,
}

impl Default for TerminalPageState {
    fn default() -> Self {
        let first_id = "tab-main".to_string();
        Self {
            tabs: vec![TerminalTab {
                id: first_id.clone(),
                title: "Terminal 1".to_string(),
                emulator: TerminalEmulator::new(100, 30),
                shell: None,
            }],
            active_tab_id: first_id.clone(),
            split_tab_id: None,
            focused_tab_id: first_id,
            font_size: 14.0,
            theme: TerminalThemeId::GlyphDefault,
            show_search: false,
            search_text: String::new(),
            show_snippets: false,
            show_secrets: false,
            tab_counter: 1,
        }
    }
}

impl TerminalPageState {
    pub fn add_tab(&mut self) -> String {
        self.tab_counter += 1;
        let id = format!("tab-{}", self.tab_counter);
        let title = format!("Terminal {}", self.tab_counter);
        self.tabs.push(TerminalTab {
            id: id.clone(),
            title,
            emulator: TerminalEmulator::new(100, 30),
            shell: None,
        });
        self.active_tab_id = id.clone();
        self.focused_tab_id = id.clone();
        id
    }

    pub fn feed_output(&mut self, tab_id: &str, bytes: &[u8]) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.emulator.feed_bytes(bytes);
        }
    }

    pub fn mark_closed(&mut self, tab_id: &str) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.shell = None;
            tab.emulator
                .feed_bytes(b"\r\n\x1b[33m[Session closed]\x1b[0m\r\n");
        }
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut TerminalTab> {
        let target = if !self.focused_tab_id.is_empty() {
            &self.focused_tab_id
        } else {
            &self.active_tab_id
        };
        self.tabs.iter_mut().find(|t| &t.id == target)
    }

    pub fn resize_pane(&mut self, tab_id: &str, cols: usize, rows: usize) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            if tab.emulator.resize(cols, rows) {
                if let Some(shell) = &tab.shell {
                    shell.resize(cols as u32, rows as u32);
                }
            }
        }
    }

    pub fn send_input(&self, tab_id: &str, bytes: Vec<u8>) {
        if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
            if let Some(shell) = &tab.shell {
                shell.write(bytes);
            }
        }
    }

    pub fn set_shell(&mut self, tab_id: &str, shell: ShellHandle) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            let cols = tab.emulator.cols() as u32;
            let rows = tab.emulator.rows() as u32;
            shell.resize(cols, rows);
            tab.shell = Some(shell);
        }
    }
}

pub fn view<'a>(
    state: &'a TerminalPageState,
    snippets: &'a [CommandSnippet],
    secrets: &'a [SecretSummary],
) -> Element<'a, TerminalPageMsg> {
    let palette = state.theme.palette();
    let bg_color = palette.background;

    // â”€â”€ Top Toolbar (48px) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    let snippets_btn = button(
        row![
            icons::zap(14.0, TEXT_LIGHT),
            text("Snippets").size(12).color(TEXT_LIGHT),
            icons::chevron_down(14.0, TEXT_SECONDARY),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding([6, 12])
    .style(secondary_button)
    .on_press(TerminalPageMsg::ToggleSnippets);

    let secrets_btn = button(
        row![
            icons::lock(14.0, TEXT_LIGHT),
            text("Secrets").size(12).color(TEXT_LIGHT),
            icons::chevron_down(14.0, TEXT_SECONDARY),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding([6, 12])
    .style(secondary_button)
    .on_press(TerminalPageMsg::ToggleSecrets);

    let theme_picker = pick_list(
        TerminalThemeId::ALL.as_slice(),
        Some(state.theme),
        TerminalPageMsg::SelectTheme,
    )
    .text_size(12)
    .padding([5, 10])
    .width(Length::Fixed(136.0))
    .style(dark_pick_list);

    let font_controls = row![
        button(icons::minus(14.0, TEXT_SECONDARY))
            .padding(5)
            .style(ghost_icon_button)
            .on_press(TerminalPageMsg::FontSmaller),
        text(format!("{}pt", state.font_size as i32))
            .size(11)
            .font(FONT_MONO)
            .color(TEXT_SECONDARY),
        button(icons::plus(14.0, TEXT_SECONDARY))
            .padding(5)
            .style(ghost_icon_button)
            .on_press(TerminalPageMsg::FontLarger),
    ]
    .spacing(4)
    .align_y(alignment::Vertical::Center);

    let split_active = state.split_tab_id.is_some();
    let split_btn = button(
        row![
            icons::columns(14.0, if split_active { iced::Color::WHITE } else { TEXT_SECONDARY }),
            text("Split")
                .size(12)
                .color(if split_active { iced::Color::WHITE } else { TEXT_SECONDARY }),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding([6, 10])
    .style(if split_active {
        brand_button
    } else {
        ghost_icon_button
    })
    .on_press(TerminalPageMsg::ToggleSplit);

    let search_btn = button(icons::search(
        15.0,
        if state.show_search {
            iced::Color::WHITE
        } else {
            TEXT_SECONDARY
        },
    ))
    .padding(6)
    .style(if state.show_search {
        brand_button
    } else {
        ghost_icon_button
    })
    .on_press(TerminalPageMsg::ToggleSearch);

    let clear_btn = button(icons::trash(15.0, RED_400))
        .padding(6)
        .style(ghost_icon_button)
        .on_press(TerminalPageMsg::ClearActive);

    let toolbar = container(
        row![
            snippets_btn,
            secrets_btn,
            icons::palette(14.0, TEXT_SECONDARY),
            theme_picker,
            v_divider(),
            font_controls,
            space().width(Length::Fill),
            split_btn,
            search_btn,
            v_divider(),
            clear_btn,
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .height(Length::Fixed(46.0))
    .width(Length::Fill)
    .padding([0, 14])
    .style(|_| container::Style {
        background: Some(iced::Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            ..Default::default()
        },
        snap: true,
        ..Default::default()
    });

    // â”€â”€ Horizontally Scrollable Tab Bar â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    let mut tab_row = row![].spacing(6).align_y(alignment::Vertical::Center);
    for tab in &state.tabs {
        let is_active = state.active_tab_id == tab.id;
        let is_split = state.split_tab_id.as_deref() == Some(tab.id.as_str());
        let id_select = tab.id.clone();
        let id_close = tab.id.clone();

        let mut inner = row![
            text(&tab.title)
                .size(12)
                .color(if is_active || is_split {
                    BRAND_300
                } else {
                    TEXT_SECONDARY
                })
                .wrapping(text::Wrapping::None),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center);

        if state.tabs.len() > 1 {
            inner = inner.push(
                button(icons::x_close(11.0, TEXT_SECONDARY))
                    .padding(2)
                    .style(ghost_icon_button)
                    .on_press(TerminalPageMsg::CloseTab(id_close)),
            );
        }

        let tab_btn = button(inner)
            .padding([5, 10])
            .style(move |_, status| {
                let bg = if is_active {
                    rgba(BRAND_500, 0.22)
                } else if is_split {
                    rgba(BRAND_500, 0.12)
                } else if matches!(status, button::Status::Hovered) {
                    DARK_700
                } else {
                    iced::Color::TRANSPARENT
                };
                button::Style {
                    background: Some(iced::Background::Color(bg)),
                    text_color: if is_active { BRAND_300 } else { TEXT_SECONDARY },
                    border: Border {
                        color: if is_active {
                            rgba(BRAND_500, 0.45)
                        } else {
                            iced::Color::TRANSPARENT
                        },
                        width: if is_active { 1.0 } else { 0.0 },
                        radius: 6.0.into(),
                    },
                    snap: true,
                    ..Default::default()
                }
            })
            .on_press(TerminalPageMsg::SelectTab(id_select));

        tab_row = tab_row.push(tab_btn);
    }

    tab_row = tab_row.push(
        button(icons::square_plus(16.0, TEXT_MUTED))
            .padding(5)
            .style(ghost_icon_button)
            .on_press(TerminalPageMsg::NewTab),
    );

    let tab_bar = container(
        scrollable(tab_row)
            .horizontal()
            .width(Length::Fill)
            .style(dark_scrollable),
    )
    .width(Length::Fill)
    .padding([6, 12])
    .style(|_| container::Style {
        background: Some(iced::Background::Color(DARK_900)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            ..Default::default()
        },
        snap: true,
        ..Default::default()
    });

    // â”€â”€ Terminal Pane(s) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    let active_tab = state
        .tabs
        .iter()
        .find(|t| t.id == state.active_tab_id)
        .or_else(|| state.tabs.first());

    let primary_pane: Element<'a, TerminalPageMsg> = if let Some(tab) = active_tab {
        let tid = tab.id.clone();
        let focused = state.focused_tab_id == tab.id;
        Element::from(
            canvas(TerminalCanvas {
                emulator: &tab.emulator,
                theme: state.theme,
                font_size: state.font_size,
                font: FONT_MONO,
                bold_font: FONT_MONO_BOLD,
                focused,
                highlight_rules: &[],
                highlights_enabled: false,
            })
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .map(move |ev| TerminalPageMsg::CanvasAction(tid.clone(), ev))
    } else {
        space().width(Length::Fill).height(Length::Fill).into()
    };

    let pane_area: Element<'a, TerminalPageMsg> = if let Some(split_id) = &state.split_tab_id {
        if let Some(split_tab) = state.tabs.iter().find(|t| &t.id == split_id) {
            let tid2 = split_tab.id.clone();
            let focused2 = state.focused_tab_id == split_tab.id;
            let secondary = Element::from(
                canvas(TerminalCanvas {
                    emulator: &split_tab.emulator,
                    theme: state.theme,
                    font_size: state.font_size,
                    font: FONT_MONO,
                    bold_font: FONT_MONO_BOLD,
                    focused: focused2,
                    highlight_rules: &[],
                    highlights_enabled: false,
                })
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .map(move |ev| TerminalPageMsg::CanvasAction(tid2.clone(), ev));

            row![
                container(primary_pane)
                    .width(Length::FillPortion(1))
                    .height(Length::Fill),
                container(text(""))
                    .width(Length::Fixed(1.0))
                    .height(Length::Fill)
                    .style(dark_800_bg),
                container(secondary)
                    .width(Length::FillPortion(1))
                    .height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            primary_pane
        }
    } else {
        primary_pane
    };

    let mut overlays: Vec<Element<'a, TerminalPageMsg>> = vec![container(pane_area)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(bg_color)),
            snap: true,
            ..Default::default()
        })
        .into()];

    if state.show_search {
        let search_box = container(
            row![
                text_input("Find...", &state.search_text)
                    .on_input(TerminalPageMsg::SearchInput)
                    .on_submit(TerminalPageMsg::SearchNext)
                    .size(13)
                    .width(Length::Fixed(180.0))
                    .style(dark_input),
                button(icons::arrow_up(14.0, TEXT_SECONDARY))
                    .padding(6)
                    .style(ghost_icon_button)
                    .on_press(TerminalPageMsg::SearchPrev),
                button(icons::arrow_down(14.0, TEXT_SECONDARY))
                    .padding(6)
                    .style(ghost_icon_button)
                    .on_press(TerminalPageMsg::SearchNext),
                button(icons::x_close(14.0, TEXT_SECONDARY))
                    .padding(6)
                    .style(ghost_icon_button)
                    .on_press(TerminalPageMsg::ToggleSearch),
            ]
            .spacing(4)
            .align_y(alignment::Vertical::Center),
        )
        .padding(6)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(DARK_800)),
            border: Border {
                color: DARK_600,
                width: 1.0,
                radius: 8.0.into(),
            },
            snap: true,
            ..Default::default()
        });

        overlays.push(
            container(search_box)
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Right)
                .padding(12)
                .into(),
        );
    }

    if state.show_snippets {
        let mut menu = column![].spacing(2);
        if snippets.is_empty() {
            menu = menu.push(
                text("No commands saved. Use the Commands tab!")
                    .size(12)
                    .color(TEXT_SECONDARY),
            );
        } else {
            for s in snippets {
                let cmd = s.cmd.clone();
                menu = menu.push(
                    button(text(&s.name).size(13).color(TEXT_LIGHT))
                        .width(Length::Fill)
                        .padding([7, 12])
                        .style(ghost_icon_button)
                        .on_press(TerminalPageMsg::RunSnippet(cmd)),
                );
            }
        }
        overlays.push(
            container(
                container(menu)
                    .width(Length::Fixed(260.0))
                    .padding(6)
                    .style(dark_card),
            )
            .padding([8, 16])
            .into(),
        );
    }

    if state.show_secrets {
        let mut menu = column![].spacing(2);
        if secrets.is_empty() {
            menu = menu.push(
                text("Vault is empty. Add secrets in the Secrets tab!")
                    .size(12)
                    .color(TEXT_SECONDARY),
            );
        } else {
            for s in secrets {
                let sid = s.id.clone();
                menu = menu.push(
                    button(
                        row![
                            text(&s.name).size(13).color(TEXT_LIGHT),
                            space().width(Length::Fill),
                            text("INJECT").size(10).font(FONT_MONO_BOLD).color(BRAND_400),
                        ]
                        .align_y(alignment::Vertical::Center),
                    )
                    .width(Length::Fill)
                    .padding([7, 12])
                    .style(ghost_icon_button)
                    .on_press(TerminalPageMsg::InjectSecret(sid)),
                );
            }
        }
        overlays.push(
            container(
                container(menu)
                    .width(Length::Fixed(260.0))
                    .padding(6)
                    .style(dark_card),
            )
            .padding([8, 130])
            .into(),
        );
    }

    let stacked = stack(overlays).width(Length::Fill).height(Length::Fill);

    column![toolbar, tab_bar, stacked]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn v_divider<'a, Message: 'a>() -> Element<'a, Message> {
    container(text(""))
        .width(Length::Fixed(1.0))
        .height(Length::Fixed(22.0))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(DARK_700)),
            snap: true,
            ..Default::default()
        })
        .into()
}

pub fn spawn_open_shell(
    session: SshSession,
    tab_id: String,
    cols: u32,
    rows: u32,
    window_id: iced::window::Id,
) {
    let bus = crate::app::app_bus_sender();
    tokio::spawn(async move {
        let (ev_tx, mut ev_rx) = tokio::sync::mpsc::unbounded_channel();
        let tab_clone = tab_id.clone();
        let bus_forward = bus.clone();
        tokio::spawn(async move {
            while let Some(ev) = ev_rx.recv().await {
                let _ = bus_forward.send(crate::app::BusEvent::Session(window_id, ev));
            }
        });

        match session.open_shell(tab_id, cols, rows, ev_tx).await {
            Ok(handle) => {
                let _ = bus.send(crate::app::BusEvent::ShellOpened {
                    window_id,
                    tab_id: tab_clone,
                    handle,
                });
            }
            Err(e) => {
                let msg = format!("\r\n\x1b[31mFailed to open shell: {e}\x1b[0m\r\n");
                let _ = bus.send(crate::app::BusEvent::Session(
                    window_id,
                    glyph_core::ssh::SessionEvent::Shell(glyph_core::ssh::ShellEvent::Data {
                        tab_id: tab_clone,
                        bytes: msg.into_bytes(),
                    }),
                ));
            }
        }
    });
}
