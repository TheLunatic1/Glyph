use crate::theme::{
    brand_button, danger_ghost_button, dark_input, dark_pick_list, dark_scrollable, glass_panel,
    logo_handle_for, rgba, secondary_button, truncate_ellipsis, BRAND_400, BRAND_500, DARK_700,
    DARK_900, FONT_MONO, FONT_MONO_BOLD, FONT_UI_BOLD, FONT_UI_SEMIBOLD, GREEN_400, RED_400,
    RED_500, TEXT_LIGHT, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400,
};
#[cfg(not(target_os = "macos"))]
use crate::theme::{window_close_button, window_control_button};
use crate::ui::icons;
use glyph_core::local_pty::LocalPtyHandle;
use glyph_core::serial::{list_serial_ports, SerialHandle, SerialPortSummary};
use glyph_core::telnet::TelnetHandle;
use glyph_terminal::{
    CompiledHighlightRule, SessionRecorder, TerminalCanvas, TerminalEmulator, TerminalEvent,
    TerminalThemeId,
};
use iced::widget::{
    button, canvas, column, container, image, mouse_area, pick_list, row, scrollable, space, text,
    text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickProtocol {
    LocalShell,
    Serial,
    Telnet,
}

impl QuickProtocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::LocalShell => "Local Terminal",
            Self::Serial => "Serial Console",
            Self::Telnet => "Telnet",
        }
    }
}

pub enum QuickTransport {
    Disconnected,
    Local(LocalPtyHandle),
    Serial(SerialHandle),
    Telnet(TelnetHandle),
}

impl QuickTransport {
    pub fn is_connected(&self) -> bool {
        !matches!(self, Self::Disconnected)
    }

    pub fn write(&self, bytes: Vec<u8>) {
        match self {
            Self::Local(h) => h.write(bytes),
            Self::Serial(h) => h.write(bytes),
            Self::Telnet(h) => h.write(bytes),
            Self::Disconnected => {}
        }
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        match self {
            Self::Local(h) => h.resize(cols, rows),
            Self::Telnet(h) => h.resize(cols, rows),
            Self::Serial(_) | Self::Disconnected => {}
        }
    }

    pub fn close(&self) {
        match self {
            Self::Local(h) => h.close(),
            Self::Serial(h) => h.close(),
            Self::Telnet(h) => h.close(),
            Self::Disconnected => {}
        }
    }
}

pub struct QuickTerminalState {
    pub protocol: QuickProtocol,
    pub transport: QuickTransport,
    pub emulator: TerminalEmulator,
    pub theme: TerminalThemeId,
    pub font_size: f32,
    pub cols: u16,
    pub rows: u16,
    pub focused: bool,
    pub highlights_enabled: bool,
    pub show_search: bool,
    pub search_text: String,
    pub status: Option<String>,
    pub error: Option<String>,
    pub recorder: Option<SessionRecorder>,

    // Serial config UI state
    pub serial_ports: Vec<SerialPortSummary>,
    pub serial_port_input: String,
    pub serial_baud: u32,
    pub serial_data_bits: u8,
    pub serial_parity: String,
    pub serial_stop_bits: u8,

    // Telnet config UI state
    pub telnet_host: String,
    pub telnet_port: String,
}

impl QuickTerminalState {
    pub fn new(protocol: QuickProtocol) -> Self {
        let ports = if protocol == QuickProtocol::Serial {
            list_serial_ports()
        } else {
            Vec::new()
        };
        let default_port = ports.first().map(|p| p.name.clone()).unwrap_or_default();

        Self {
            protocol,
            transport: QuickTransport::Disconnected,
            emulator: TerminalEmulator::new(100, 30),
            theme: TerminalThemeId::GlyphDefault,
            font_size: 13.5,
            cols: 100,
            rows: 30,
            focused: true,
            highlights_enabled: true,
            show_search: false,
            search_text: String::new(),
            status: None,
            error: None,
            recorder: None,
            serial_ports: ports,
            serial_port_input: default_port,
            serial_baud: 115_200,
            serial_data_bits: 8,
            serial_parity: "None".to_string(),
            serial_stop_bits: 1,
            telnet_host: String::new(),
            telnet_port: "23".to_string(),
        }
    }

    pub fn title(&self) -> String {
        match self.protocol {
            QuickProtocol::LocalShell => "Glyph — Local Terminal".to_string(),
            QuickProtocol::Serial => {
                if self.serial_port_input.is_empty() {
                    "Glyph — Serial Console".to_string()
                } else {
                    format!("Glyph — Serial ({})", self.serial_port_input)
                }
            }
            QuickProtocol::Telnet => {
                if self.telnet_host.trim().is_empty() {
                    "Glyph — Telnet".to_string()
                } else {
                    format!(
                        "Glyph — Telnet ({}:{})",
                        self.telnet_host.trim(),
                        self.telnet_port.trim()
                    )
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum QuickTerminalMsg {
    // Window controls
    StartDrag,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    MinimizeWindow,
    MaximizeWindow,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    CloseWindow,

    // Toolbar controls
    SelectTheme(TerminalThemeId),
    FontSmaller,
    FontLarger,
    ToggleHighlights,
    ToggleRecording,
    ExportPlainLog,
    ToggleSearch,
    SearchInput(String),
    SearchNext,
    SearchPrev,
    ClearScreen,
    ReconnectLocal,

    // Serial config
    RefreshSerialPorts,
    SelectSerialPort(SerialPortSummary),
    SerialPortInput(String),
    SelectBaud(u32),
    SelectDataBits(u8),
    SelectParity(String),
    SelectStopBits(u8),
    ConnectSerial,
    DisconnectTransport,

    // Telnet config
    TelnetHostInput(String),
    TelnetPortInput(String),
    ConnectTelnet,

    // Terminal canvas
    CanvasAction(TerminalEvent),
}

fn bar_bg(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DARK_900)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 0.0.into(),
        },
        text_color: Some(TEXT_PRIMARY),
        snap: true,
        ..Default::default()
    }
}

fn active_pill_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        if active {
            button::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.25))),
                text_color: BRAND_400,
                border: Border {
                    color: BRAND_500,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                snap: true,
                ..Default::default()
            }
        } else {
            secondary_button(theme, status)
        }
    }
}

pub fn view<'a>(
    st: &'a QuickTerminalState,
    highlight_rules: &'a [CompiledHighlightRule],
) -> Element<'a, QuickTerminalMsg> {
    let mut left_row = row![];
    #[cfg(target_os = "macos")]
    {
        left_row = left_row.push(space().width(Length::Fixed(74.0)));
    }
    #[cfg(not(target_os = "macos"))]
    {
        left_row = left_row.push(space().width(10));
    }
    left_row = left_row
        .push(
            image(logo_handle_for(20))
                .width(20)
                .height(20)
                .filter_method(image::FilterMethod::Linear),
        )
        .push(space().width(8))
        .push(
            text(truncate_ellipsis(&st.title(), 48))
                .size(13)
                .font(FONT_UI_SEMIBOLD)
                .color(TEXT_PRIMARY),
        )
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fixed(34.0));

    let drag_header = mouse_area(left_row)
        .on_press(QuickTerminalMsg::StartDrag)
        .on_double_click(QuickTerminalMsg::MaximizeWindow);

    #[cfg(not(target_os = "macos"))]
    let titlebar_row = {
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

        let close_glyph = container(icons::x_close_inherit(14.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        row![
            drag_header,
            button(minimize_glyph)
                .on_press(QuickTerminalMsg::MinimizeWindow)
                .width(Length::Fixed(42.0))
                .height(Length::Fixed(34.0))
                .padding(0)
                .style(window_control_button),
            button(maximize_glyph)
                .on_press(QuickTerminalMsg::MaximizeWindow)
                .width(Length::Fixed(42.0))
                .height(Length::Fixed(34.0))
                .padding(0)
                .style(window_control_button),
            button(close_glyph)
                .on_press(QuickTerminalMsg::CloseWindow)
                .width(Length::Fixed(42.0))
                .height(Length::Fixed(34.0))
                .padding(0)
                .style(window_close_button),
        ]
        .align_y(alignment::Vertical::Center)
    };

    #[cfg(target_os = "macos")]
    let titlebar_row = row![drag_header].align_y(alignment::Vertical::Center);

    let titlebar = container(titlebar_row)
        .padding(0)
        .width(Length::Fill)
        .height(Length::Fixed(34.0))
        .style(bar_bg);

    let connected = st.transport.is_connected();
    let status_dot_color = if connected { GREEN_400 } else { YELLOW_400 };
    let recording = st.recorder.is_some();

    let mut toolbar_row = row![
        icons::circle_dot(14.0, status_dot_color),
        space().width(4),
        text(st.protocol.label())
            .size(12)
            .font(FONT_UI_BOLD)
            .color(TEXT_PRIMARY),
        space().width(8),
        button(
            row![
                icons::radio(13.0, if recording { RED_500 } else { TEXT_SECONDARY }),
                space().width(5),
                text(if recording { "Rec On" } else { "Record" })
                    .size(12)
                    .color(if recording { RED_400 } else { TEXT_LIGHT }),
            ]
            .align_y(alignment::Vertical::Center),
        )
        .on_press(QuickTerminalMsg::ToggleRecording)
        .padding([5, 9])
        .style(if recording {
            danger_ghost_button
        } else {
            secondary_button
        }),
        button(
            row![
                icons::highlighter(
                    13.0,
                    if st.highlights_enabled {
                        BRAND_400
                    } else {
                        TEXT_SECONDARY
                    }
                ),
                space().width(5),
                text("Highlights").size(12),
            ]
            .align_y(alignment::Vertical::Center),
        )
        .on_press(QuickTerminalMsg::ToggleHighlights)
        .padding([5, 9])
        .style(active_pill_button(st.highlights_enabled)),
        button(
            row![
                icons::file_down(13.0, TEXT_LIGHT),
                space().width(5),
                text("Export .log").size(12).color(TEXT_LIGHT),
            ]
            .align_y(alignment::Vertical::Center),
        )
        .on_press(QuickTerminalMsg::ExportPlainLog)
        .padding([5, 9])
        .style(secondary_button),
        button(text("Find").size(12))
            .on_press(QuickTerminalMsg::ToggleSearch)
            .padding([5, 9])
            .style(active_pill_button(st.show_search)),
        button(text("Clear").size(12).color(TEXT_LIGHT))
            .on_press(QuickTerminalMsg::ClearScreen)
            .padding([5, 9])
            .style(secondary_button),
        space().width(8),
        pick_list(
            TerminalThemeId::ALL.as_slice(),
            Some(st.theme),
            QuickTerminalMsg::SelectTheme
        )
        .text_size(12)
        .padding([4, 8])
        .style(dark_pick_list),
        button(text("A-").size(11).color(TEXT_LIGHT))
            .on_press(QuickTerminalMsg::FontSmaller)
            .padding([5, 8])
            .style(secondary_button),
        text(format!("{:.0}", st.font_size))
            .size(11)
            .font(FONT_MONO)
            .color(TEXT_SECONDARY),
        button(text("A+").size(11).color(TEXT_LIGHT))
            .on_press(QuickTerminalMsg::FontLarger)
            .padding([5, 8])
            .style(secondary_button),
    ]
    .spacing(6)
    .align_y(alignment::Vertical::Center);

    if st.protocol == QuickProtocol::LocalShell && !connected {
        toolbar_row = toolbar_row.push(
            button(text("Restart Shell").size(12).color(Color::WHITE))
                .on_press(QuickTerminalMsg::ReconnectLocal)
                .padding([5, 10])
                .style(brand_button),
        );
    }

    let toolbar = container(
        scrollable(toolbar_row)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .style(dark_scrollable),
    )
    .padding([6, 10])
    .width(Length::Fill)
    .style(bar_bg);

    let mut root = column![titlebar, toolbar]
        .width(Length::Fill)
        .height(Length::Fill);

    match st.protocol {
        QuickProtocol::Serial => {
            let bauds: Vec<u32> = vec![9600, 19200, 38400, 57600, 115_200, 921_600];
            let bits: Vec<u8> = vec![8, 7];
            let stops: Vec<u8> = vec![1, 2];
            let parities = vec!["None".to_string(), "Even".to_string(), "Odd".to_string()];
            let selected_port = st
                .serial_ports
                .iter()
                .find(|p| p.name == st.serial_port_input)
                .cloned();

            let mut serial_row = row![
                icons::cable(15.0, BRAND_400),
                pick_list(
                    st.serial_ports.clone(),
                    selected_port,
                    QuickTerminalMsg::SelectSerialPort
                )
                .placeholder("Detected Ports")
                .text_size(12)
                .padding([4, 8])
                .style(dark_pick_list),
                text_input("Port (e.g. COM3 or /dev/ttyUSB0)", &st.serial_port_input)
                    .on_input(QuickTerminalMsg::SerialPortInput)
                    .size(12)
                    .padding([5, 8])
                    .width(Length::Fixed(180.0))
                    .style(dark_input),
                button(text("Refresh").size(12).color(TEXT_LIGHT))
                    .on_press(QuickTerminalMsg::RefreshSerialPorts)
                    .padding([5, 8])
                    .style(secondary_button),
                pick_list(bauds, Some(st.serial_baud), QuickTerminalMsg::SelectBaud)
                    .text_size(12)
                    .padding([4, 8])
                    .style(dark_pick_list),
                pick_list(
                    bits,
                    Some(st.serial_data_bits),
                    QuickTerminalMsg::SelectDataBits
                )
                .text_size(12)
                .padding([4, 6])
                .style(dark_pick_list),
                pick_list(
                    parities,
                    Some(st.serial_parity.clone()),
                    QuickTerminalMsg::SelectParity
                )
                .text_size(12)
                .padding([4, 8])
                .style(dark_pick_list),
                pick_list(
                    stops,
                    Some(st.serial_stop_bits),
                    QuickTerminalMsg::SelectStopBits
                )
                .text_size(12)
                .padding([4, 6])
                .style(dark_pick_list),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center);

            if connected {
                serial_row = serial_row.push(
                    button(text("Disconnect").size(12))
                        .on_press(QuickTerminalMsg::DisconnectTransport)
                        .padding([5, 10])
                        .style(danger_ghost_button),
                );
            } else {
                serial_row = serial_row.push(
                    button(text("Connect Serial").size(12).color(Color::WHITE))
                        .on_press(QuickTerminalMsg::ConnectSerial)
                        .padding([5, 10])
                        .style(brand_button),
                );
            }

            root = root.push(
                container(
                    scrollable(serial_row)
                        .direction(scrollable::Direction::Horizontal(
                            scrollable::Scrollbar::new().width(4).scroller_width(4),
                        ))
                        .style(dark_scrollable),
                )
                .padding([6, 10])
                .width(Length::Fill)
                .style(glass_panel),
            );
        }
        QuickProtocol::Telnet => {
            let mut telnet_row = row![
                icons::globe(15.0, BRAND_400),
                text("Host:").size(12).color(TEXT_SECONDARY),
                text_input("192.168.1.1 or switch.local", &st.telnet_host)
                    .on_input(QuickTerminalMsg::TelnetHostInput)
                    .on_submit(QuickTerminalMsg::ConnectTelnet)
                    .size(12)
                    .padding([5, 9])
                    .width(Length::Fixed(240.0))
                    .style(dark_input),
                text("Port:").size(12).color(TEXT_SECONDARY),
                text_input("23", &st.telnet_port)
                    .on_input(QuickTerminalMsg::TelnetPortInput)
                    .on_submit(QuickTerminalMsg::ConnectTelnet)
                    .size(12)
                    .padding([5, 9])
                    .width(Length::Fixed(72.0))
                    .style(dark_input),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center);

            if connected {
                telnet_row = telnet_row.push(
                    button(text("Disconnect").size(12))
                        .on_press(QuickTerminalMsg::DisconnectTransport)
                        .padding([5, 12])
                        .style(danger_ghost_button),
                );
            } else {
                telnet_row = telnet_row.push(
                    button(text("Connect Telnet").size(12).color(Color::WHITE))
                        .on_press(QuickTerminalMsg::ConnectTelnet)
                        .padding([5, 12])
                        .style(brand_button),
                );
            }

            root = root.push(
                container(telnet_row)
                    .padding([6, 10])
                    .width(Length::Fill)
                    .style(glass_panel),
            );
        }
        QuickProtocol::LocalShell => {}
    }

    if st.show_search {
        root = root.push(
            container(
                row![
                    text_input("Search terminal buffer...", &st.search_text)
                        .on_input(QuickTerminalMsg::SearchInput)
                        .on_submit(QuickTerminalMsg::SearchNext)
                        .size(12)
                        .padding([5, 9])
                        .width(Length::Fixed(260.0))
                        .style(dark_input),
                    button(text("Prev").size(12).color(TEXT_LIGHT))
                        .on_press(QuickTerminalMsg::SearchPrev)
                        .padding([5, 10])
                        .style(secondary_button),
                    button(text("Next").size(12).color(TEXT_LIGHT))
                        .on_press(QuickTerminalMsg::SearchNext)
                        .padding([5, 10])
                        .style(secondary_button),
                    button(text("Close").size(12).color(TEXT_LIGHT))
                        .on_press(QuickTerminalMsg::ToggleSearch)
                        .padding([5, 10])
                        .style(secondary_button),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding([6, 10])
            .width(Length::Fill)
            .style(bar_bg),
        );
    }

    if let Some(err) = &st.error {
        root = root.push(
            container(text(err).size(12).color(RED_400))
                .padding([6, 10])
                .width(Length::Fill)
                .style(bar_bg),
        );
    } else if let Some(msg) = &st.status {
        root = root.push(
            container(text(msg).size(12).color(GREEN_400))
                .padding([5, 10])
                .width(Length::Fill)
                .style(bar_bg),
        );
    }

    let term_canvas = canvas(TerminalCanvas {
        emulator: &st.emulator,
        theme: st.theme,
        font_size: st.font_size,
        font: FONT_MONO,
        bold_font: FONT_MONO_BOLD,
        focused: st.focused,
        highlight_rules,
        highlights_enabled: st.highlights_enabled,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    let canvas_el: Element<'a, QuickTerminalMsg> =
        Element::from(term_canvas).map(QuickTerminalMsg::CanvasAction);

    root = root.push(
        container(canvas_el)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(4)
            .style(|_| container::Style {
                background: Some(Background::Color(DARK_900)),
                snap: true,
                ..Default::default()
            }),
    );

    root.into()
}
