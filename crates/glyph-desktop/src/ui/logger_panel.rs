use crate::theme::{
    brand_button, danger_ghost_button, dark_input, dark_scrollable, modal_backdrop, modal_card,
    pill_badge, rgba, secondary_button, BLUE_400, BRAND_300, BRAND_400, BRAND_500, DARK_700,
    DARK_800, DARK_900, FONT_MONO, FONT_MONO_MEDIUM, FONT_UI_BOLD, FONT_UI_SEMIBOLD, GREEN_400,
    RED_400, TEXT_DIM, TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400,
};
use crate::ui::icons;
use chrono::Local;
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, space, stack, text, text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub const MAX_LOG_ENTRIES: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Success,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn badge(self) -> &'static str {
        match self {
            Self::Success => "SUCCESS",
            Self::Info => "INFO",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Success => GREEN_400,
            Self::Info => BLUE_400,
            Self::Warning => YELLOW_400,
            Self::Error => RED_400,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFilter {
    #[default]
    All,
    Success,
    Info,
    Warning,
    Error,
}

impl LogFilter {
    pub fn matches(self, level: LogLevel) -> bool {
        match self {
            Self::All => true,
            Self::Success => level == LogLevel::Success,
            Self::Info => level == LogLevel::Info,
            Self::Warning => level == LogLevel::Warning,
            Self::Error => level == LogLevel::Error,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Success => "Success",
            Self::Info => "Info",
            Self::Warning => "Warnings",
            Self::Error => "Errors",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub timestamp_hms: String,
    pub timestamp_full: String,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ActivityLoggerState {
    pub open: bool,
    pub entries: VecDeque<LogEntry>,
    pub filter: LogFilter,
    pub search: String,
    pub unread_warnings: usize,
    pub unread_errors: usize,
    pub status_hint: Option<String>,
    pub log_file: Option<PathBuf>,
}

impl Default for ActivityLoggerState {
    fn default() -> Self {
        Self::new(default_log_file_path())
    }
}

impl ActivityLoggerState {
    pub fn new(log_file: Option<PathBuf>) -> Self {
        Self {
            open: false,
            entries: VecDeque::with_capacity(128),
            filter: LogFilter::All,
            search: String::new(),
            unread_warnings: 0,
            unread_errors: 0,
            status_hint: None,
            log_file,
        }
    }

    /// Appends a sanitized activity entry to the in-memory ring buffer and optional log file.
    /// Callers must never pass credentials, passwords, passphrases, or private keys here.
    pub fn push(&mut self, level: LogLevel, source: impl Into<String>, message: impl Into<String>) {
        let now = Local::now();
        let entry = LogEntry {
            timestamp_hms: now.format("%H:%M:%S").to_string(),
            timestamp_full: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            level,
            source: source.into(),
            message: message.into(),
        };

        if !self.open {
            match level {
                LogLevel::Warning => self.unread_warnings += 1,
                LogLevel::Error => self.unread_errors += 1,
                LogLevel::Success | LogLevel::Info => {}
            }
        }

        if let Some(path) = &self.log_file {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(
                    file,
                    "[{}] [{}] [{}] {}",
                    entry.timestamp_full,
                    entry.level.badge(),
                    entry.source,
                    entry.message
                );
            }
        }

        if self.entries.len() >= MAX_LOG_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn open_panel(&mut self) {
        self.open = true;
        self.unread_warnings = 0;
        self.unread_errors = 0;
        self.status_hint = None;
    }

    pub fn close_panel(&mut self) {
        self.open = false;
        self.status_hint = None;
    }

    pub fn toggle(&mut self) {
        if self.open {
            self.close_panel();
        } else {
            self.open_panel();
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.unread_warnings = 0;
        self.unread_errors = 0;
        self.status_hint = Some("Cleared session activity logs.".to_string());
    }

    pub fn unread_alert_count(&self) -> usize {
        self.unread_warnings + self.unread_errors
    }

    pub fn count_by_level(&self, level: LogLevel) -> usize {
        self.entries.iter().filter(|e| e.level == level).count()
    }

    pub fn filtered_entries(&self) -> Vec<&LogEntry> {
        let q = self.search.trim().to_lowercase();
        self.entries
            .iter()
            .rev()
            .filter(|entry| {
                if !self.filter.matches(entry.level) {
                    return false;
                }
                if q.is_empty() {
                    return true;
                }
                entry.source.to_lowercase().contains(&q)
                    || entry.message.to_lowercase().contains(&q)
                    || entry.level.badge().to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn format_plain_text(&self) -> String {
        let mut out = String::new();
        for entry in self.filtered_entries() {
            out.push_str(&format!(
                "[{}] [{}] [{}] {}\n",
                entry.timestamp_full,
                entry.level.badge(),
                entry.source,
                entry.message
            ));
        }
        out
    }
}

pub fn default_log_file_path() -> Option<PathBuf> {
    directories::UserDirs::new().map(|u| u.home_dir().join(".glyph").join("activity.log"))
}

#[derive(Debug, Clone)]
pub enum LoggerMsg {
    Close,
    Noop,
    SetFilter(LogFilter),
    SearchChanged(String),
    ClearLogs,
    CopyLogs,
}

fn filter_tab<'a>(filter: LogFilter, active: LogFilter, count: usize) -> Element<'a, LoggerMsg> {
    let is_active = filter == active;
    let label_color = if is_active { Color::WHITE } else { TEXT_LIGHT };
    let count_color = match filter {
        LogFilter::Success => GREEN_400,
        LogFilter::Info => BLUE_400,
        LogFilter::Warning => YELLOW_400,
        LogFilter::Error => RED_400,
        LogFilter::All => BRAND_300,
    };
    button(
        row![
            text(filter.label())
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(label_color),
            container(
                text(count.to_string())
                    .size(10)
                    .font(FONT_MONO_MEDIUM)
                    .color(if is_active { Color::WHITE } else { count_color }),
            )
            .padding([1, 6])
            .style(pill_badge(
                rgba(count_color, if is_active { 0.28 } else { 0.14 }),
                rgba(count_color, 0.4),
                count_color,
            )),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding([6, 10])
    .style(if is_active {
        brand_button
    } else {
        secondary_button
    })
    .on_press(LoggerMsg::SetFilter(filter))
    .into()
}

pub fn view<'a>(st: &'a ActivityLoggerState) -> Element<'a, LoggerMsg> {
    let filtered = st.filtered_entries();

    let header =
        row![
            container(icons::scroll_text(18.0, BRAND_300))
                .padding(8)
                .style(|_| container::Style {
                    background: Some(Background::Color(rgba(BRAND_500, 0.16))),
                    border: Border {
                        color: rgba(BRAND_400, 0.35),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    snap: true,
                    ..Default::default()
                }),
            column![
            text("Activity Logger")
                .size(16)
                .font(FONT_UI_BOLD)
                .color(TEXT_PRIMARY),
            text("Live record of connections, SFTP, tunnels, containers, updates, and MCP activity")
                .size(12)
                .color(TEXT_SECONDARY),
        ]
            .spacing(2),
            space().width(Length::Fill),
            button(
                row![
                    icons::copy(13.0, TEXT_LIGHT),
                    text("Copy").size(12).color(TEXT_LIGHT),
                ]
                .spacing(5)
                .align_y(alignment::Vertical::Center),
            )
            .padding([6, 10])
            .style(secondary_button)
            .on_press(LoggerMsg::CopyLogs),
            button(
                row![
                    icons::trash(13.0, RED_400),
                    text("Clear").size(12).color(RED_400),
                ]
                .spacing(5)
                .align_y(alignment::Vertical::Center),
            )
            .padding([6, 10])
            .style(danger_ghost_button)
            .on_press(LoggerMsg::ClearLogs),
            button(icons::x_close(16.0, TEXT_SECONDARY))
                .padding(6)
                .style(secondary_button)
                .on_press(LoggerMsg::Close),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);

    let filter_bar = row![
        filter_tab(LogFilter::All, st.filter, st.entries.len()),
        filter_tab(
            LogFilter::Success,
            st.filter,
            st.count_by_level(LogLevel::Success)
        ),
        filter_tab(
            LogFilter::Info,
            st.filter,
            st.count_by_level(LogLevel::Info)
        ),
        filter_tab(
            LogFilter::Warning,
            st.filter,
            st.count_by_level(LogLevel::Warning)
        ),
        filter_tab(
            LogFilter::Error,
            st.filter,
            st.count_by_level(LogLevel::Error)
        ),
        space().width(Length::Fill),
        text_input("Filter source or message...", &st.search)
            .on_input(LoggerMsg::SearchChanged)
            .size(12)
            .padding([6, 10])
            .width(Length::Fixed(230.0))
            .style(dark_input),
    ]
    .spacing(6)
    .align_y(alignment::Vertical::Center);

    let mut list_col = column![].spacing(6).width(Length::Fill);
    if filtered.is_empty() {
        list_col = list_col.push(
            container(
                column![
                    icons::scroll_text(32.0, TEXT_DIM),
                    text(if st.entries.is_empty() {
                        "No activity logged yet"
                    } else {
                        "No log entries match the active filter"
                    })
                    .size(14)
                    .font(FONT_UI_SEMIBOLD)
                    .color(TEXT_PRIMARY),
                    text("Successes, warnings, errors, and background events will appear here automatically.")
                        .size(12)
                        .color(TEXT_SECONDARY),
                ]
                .spacing(6)
                .align_x(alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(36)
            .align_x(alignment::Horizontal::Center),
        );
    } else {
        for entry in filtered {
            let color = entry.level.color();
            let card_row = container(
                row![
                    text(&entry.timestamp_hms)
                        .size(11)
                        .font(FONT_MONO)
                        .color(TEXT_MUTED)
                        .width(Length::Fixed(64.0)),
                    container(
                        text(entry.level.badge())
                            .size(10)
                            .font(FONT_MONO_MEDIUM)
                            .color(color),
                    )
                    .padding([2, 7])
                    .width(Length::Fixed(68.0))
                    .align_x(alignment::Horizontal::Center)
                    .style(pill_badge(
                        rgba(color, 0.15),
                        rgba(color, 0.4),
                        color
                    )),
                    container(
                        text(&entry.source)
                            .size(11)
                            .font(FONT_MONO_MEDIUM)
                            .color(BRAND_300),
                    )
                    .padding([2, 7])
                    .style(pill_badge(
                        rgba(BRAND_500, 0.12),
                        rgba(BRAND_400, 0.28),
                        BRAND_300,
                    )),
                    text(&entry.message)
                        .size(12)
                        .color(TEXT_PRIMARY)
                        .width(Length::Fill),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([8, 10])
            .style(move |_| container::Style {
                background: Some(Background::Color(DARK_900)),
                border: Border {
                    color: if entry.level == LogLevel::Error {
                        rgba(RED_400, 0.35)
                    } else if entry.level == LogLevel::Warning {
                        rgba(YELLOW_400, 0.30)
                    } else {
                        DARK_700
                    },
                    width: 1.0,
                    radius: 8.0.into(),
                },
                snap: true,
                ..Default::default()
            });
            list_col = list_col.push(card_row);
        }
    }

    let path_label = st
        .log_file
        .as_ref()
        .map(|p| format!("Persisting to {}", p.display()))
        .unwrap_or_else(|| "In-memory session log".to_string());

    let footer = row![
        text(path_label).size(11).font(FONT_MONO).color(TEXT_MUTED),
        space().width(Length::Fill),
        text(
            st.status_hint
                .as_deref()
                .unwrap_or("Shortcut: Ctrl+Shift+L")
        )
        .size(11)
        .color(if st.status_hint.is_some() {
            GREEN_400
        } else {
            TEXT_SECONDARY
        }),
    ]
    .align_y(alignment::Vertical::Center);

    let card = mouse_area(
        container(
            column![
                header,
                filter_bar,
                scrollable(container(list_col).padding([2, 6]))
                    .height(Length::Fixed(410.0))
                    .width(Length::Fill)
                    .style(dark_scrollable),
                footer,
            ]
            .spacing(12),
        )
        .width(Length::Fixed(860.0))
        .max_width(920.0)
        .padding(18)
        .style(modal_card),
    )
    .on_press(LoggerMsg::Noop);

    let backdrop = mouse_area(
        container(space().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(modal_backdrop),
    )
    .on_press(LoggerMsg::Close);

    stack![
        backdrop,
        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    ]
    .into()
}

#[allow(dead_code)]
pub fn modal_card_border() -> Color {
    DARK_800
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_entries_at_max_limit() {
        let mut st = ActivityLoggerState::new(None);
        for i in 0..(MAX_LOG_ENTRIES + 50) {
            st.push(LogLevel::Info, "Test", format!("entry-{i}"));
        }
        assert_eq!(st.entries.len(), MAX_LOG_ENTRIES);
        assert_eq!(st.entries.front().unwrap().message, "entry-50");
        assert_eq!(
            st.entries.back().unwrap().message,
            format!("entry-{}", MAX_LOG_ENTRIES + 49)
        );
    }

    #[test]
    fn tracks_unread_warnings_and_errors_only_when_closed() {
        let mut st = ActivityLoggerState::new(None);
        st.push(LogLevel::Success, "SSH", "Connected");
        st.push(LogLevel::Info, "MCP", "Tool ran");
        st.push(LogLevel::Warning, "Tunnel", "Port busy");
        st.push(LogLevel::Error, "SFTP", "Permission denied");

        assert_eq!(st.unread_warnings, 1);
        assert_eq!(st.unread_errors, 1);
        assert_eq!(st.unread_alert_count(), 2);

        st.open_panel();
        assert_eq!(st.unread_alert_count(), 0);

        st.push(LogLevel::Error, "SSH", "Timeout while open");
        assert_eq!(st.unread_alert_count(), 0);
    }

    #[test]
    fn filters_by_level_and_search_query() {
        let mut st = ActivityLoggerState::new(None);
        st.push(LogLevel::Success, "SSH", "Connected to prod-web");
        st.push(LogLevel::Error, "SSH", "Auth failed for db-01");
        st.push(LogLevel::Error, "SFTP", "Checksum mismatch");

        st.filter = LogFilter::Error;
        assert_eq!(st.filtered_entries().len(), 2);

        st.search = "db-01".to_string();
        let res = st.filtered_entries();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].source, "SSH");
    }

    #[test]
    fn formats_plain_text_log_lines() {
        let mut st = ActivityLoggerState::new(None);
        st.push(LogLevel::Success, "Updater", "Checked release v3.0.1");
        let text = st.format_plain_text();
        assert!(text.contains("[SUCCESS] [Updater] Checked release v3.0.1"));
    }
}
