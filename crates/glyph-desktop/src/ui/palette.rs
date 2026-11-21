use crate::theme::{
    dark_input, dark_scrollable, modal_card, rgba, secondary_button, BRAND_400, BRAND_500,
    DARK_700, TEXT_LIGHT, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons;
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, space, stack, text, text_input,
};
use iced::{alignment, window, Background, Border, Color, Element, Length, Theme};

#[derive(Debug, Clone)]
pub enum PaletteAction {
    ConnectServer(String),
    FocusServerWindow(window::Id),
    FilterGroup(Option<String>),
    OpenLocalTerminal,
    OpenSerial,
    OpenTelnet,
    OpenIdentities,
    OpenSshKeys,
    OpenHostImport,
    OpenSettings,
    OpenSyncSettings,
    LockVault,
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub title: String,
    pub subtitle: String,
    pub badge: String,
    pub action: PaletteAction,
}

#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub selected_idx: usize,
    pub items: Vec<PaletteItem>,
}

#[derive(Debug, Clone)]
pub enum PaletteMsg {
    Close,
    Noop,
    QueryChanged(String),
    ActivateSelected,
    ActivateIndex(usize),
}

pub fn filtered_indices(st: &CommandPaletteState) -> Vec<usize> {
    let q = st.query.trim().to_lowercase();
    if q.is_empty() {
        return (0..st.items.len()).take(32).collect();
    }
    let words: Vec<&str> = q.split_whitespace().collect();
    st.items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            let hay = format!(
                "{} {} {}",
                item.title.to_lowercase(),
                item.subtitle.to_lowercase(),
                item.badge.to_lowercase()
            );
            words.iter().all(|w| hay.contains(w)).then_some(idx)
        })
        .take(32)
        .collect()
}

fn backdrop_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.02, 0.024, 0.045, 0.72))),
        snap: true,
        ..Default::default()
    }
}

fn item_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let bg = if selected {
            rgba(BRAND_500, 0.22)
        } else if hovered {
            rgba(BRAND_500, 0.12)
        } else {
            Color::TRANSPARENT
        };
        let border_color = if selected {
            rgba(BRAND_400, 0.55)
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: TEXT_PRIMARY,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 8.0.into(),
            },
            snap: true,
            ..Default::default()
        }
    }
}

fn badge_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(rgba(BRAND_500, 0.16))),
        border: Border {
            color: rgba(BRAND_500, 0.32),
            width: 1.0,
            radius: 6.0.into(),
        },
        text_color: Some(BRAND_400),
        snap: true,
        ..Default::default()
    }
}

pub fn view<'a>(st: &'a CommandPaletteState) -> Element<'a, PaletteMsg> {
    let matches = filtered_indices(st);
    let clamped_sel = if matches.is_empty() {
        0
    } else {
        st.selected_idx.min(matches.len() - 1)
    };

    let header = row![
        icons::command(18.0, BRAND_400),
        space().width(8),
        text_input("Jump to host, group, local shell, serial, or action...", &st.query)
            .id("palette-search-input")
            .on_input(PaletteMsg::QueryChanged)
            .on_submit(PaletteMsg::ActivateSelected)
            .padding([9, 12])
            .size(14)
            .width(Length::Fill)
            .style(dark_input),
        space().width(8),
        button(text("Esc").size(11).color(TEXT_LIGHT))
            .on_press(PaletteMsg::Close)
            .padding([5, 10])
            .style(secondary_button),
    ]
    .align_y(alignment::Vertical::Center);

    let mut list_col = column![].spacing(4).width(Length::Fill);
    if matches.is_empty() {
        list_col = list_col.push(
            container(
                text("No matching hosts or actions.")
                    .size(13)
                    .color(TEXT_SECONDARY),
            )
            .padding([18, 12])
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Center),
        );
    } else {
        for (pos, idx) in matches.iter().enumerate() {
            let item = &st.items[*idx];
            let selected = pos == clamped_sel;
            let row_ui = row![
                column![
                    text(&item.title).size(13).color(TEXT_PRIMARY),
                    text(&item.subtitle).size(11).color(TEXT_SECONDARY),
                ]
                .width(Length::Fill),
                container(text(&item.badge).size(11).color(BRAND_400))
                    .padding([3, 8])
                    .style(badge_style),
            ]
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill);

            list_col = list_col.push(
                button(row_ui)
                    .on_press(PaletteMsg::ActivateIndex(pos))
                    .padding([8, 10])
                    .width(Length::Fill)
                    .style(item_button(selected)),
            );
        }
    }

    let modal_box = mouse_area(
        container(
            column![
                header,
                space().height(10),
                scrollable(list_col)
                    .height(Length::Fixed(320.0))
                    .style(dark_scrollable),
                space().height(8),
                container(
                    row![
                        text("Up/Down navigate").size(11).color(TEXT_SECONDARY),
                        space().width(Length::Fill),
                        text("Enter open").size(11).color(TEXT_SECONDARY),
                    ]
                    .width(Length::Fill),
                )
                .padding([6, 4])
                .style(|_| container::Style {
                    border: Border {
                        color: DARK_700,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    snap: true,
                    ..Default::default()
                }),
            ]
            .width(Length::Fill),
        )
        .width(Length::Fixed(620.0))
        .padding(16)
        .style(modal_card),
    )
    .on_press(PaletteMsg::Noop);

    let backdrop = mouse_area(
        container(space().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(backdrop_style),
    )
    .on_press(PaletteMsg::Close);

    let foreground = container(modal_box)
        .width(Length::Fill)
        .padding([56, 24])
        .align_x(alignment::Horizontal::Center);

    stack![backdrop, foreground]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
