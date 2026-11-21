use crate::theme::{
    brand_button, danger_ghost_button, dark_card, dark_input, dark_scrollable, glass_panel,
    pill_badge, rgba, secondary_button, BRAND_400, BRAND_500, DARK_700, DARK_900, FONT_MONO,
    FONT_MONO_BOLD, GREEN_400, GREEN_500, RED_400, TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY,
};
use crate::ui::icons;
use glyph_core::models::CommandSnippet;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{alignment, Background, Border, Color, Element, Length};

#[derive(Debug, Clone)]
pub enum CommandsPageMsg {
    NameInput(String),
    CmdInput(String),
    AddSubmit,
    Run(u64, String),
    Delete(u64),
}

#[derive(Debug, Clone, Default)]
pub struct CommandsPageState {
    pub new_name: String,
    pub new_cmd: String,
    pub error: Option<String>,
}

pub fn view<'a>(
    st: &'a CommandsPageState,
    commands: &'a [CommandSnippet],
) -> Element<'a, CommandsPageMsg> {
    let header = row![
        container(icons::code(20.0, BRAND_400))
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
            text("Saved Commands")
                .size(20)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("Quickly execute frequent shell operations on this server")
                .size(12)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let add_form = container(
        column![
            text("Add Command Snippet")
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(TEXT_LIGHT),
            row![
                text_input("Snippet Name (e.g. Reload Nginx)", &st.new_name)
                    .on_input(CommandsPageMsg::NameInput)
                    .on_submit(CommandsPageMsg::AddSubmit)
                    .padding([9, 12])
                    .size(13)
                    .width(Length::FillPortion(2))
                    .style(dark_input),
                text_input("Shell Command (e.g. sudo systemctl reload nginx)", &st.new_cmd)
                    .on_input(CommandsPageMsg::CmdInput)
                    .on_submit(CommandsPageMsg::AddSubmit)
                    .padding([9, 12])
                    .size(13)
                    .font(FONT_MONO)
                    .width(Length::FillPortion(3))
                    .style(dark_input),
                button(
                    row![
                        icons::plus(14.0, Color::WHITE),
                        text("Add").size(13).font(FONT_MONO_BOLD).color(Color::WHITE),
                    ]
                    .spacing(6)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([9, 16])
                .style(brand_button)
                .on_press(CommandsPageMsg::AddSubmit),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(10),
    )
    .width(Length::Fill)
    .padding(16)
    .style(glass_panel);

    let mut body = column![header, add_form].spacing(16).padding(24);

    if let Some(err) = &st.error {
        body = body.push(
            container(text(err).size(12).color(RED_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
        );
    }

    body = body.push(
        row![
            icons::zap(15.0, BRAND_400),
            text("Sorted by Most Used")
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(TEXT_LIGHT),
            space().width(Length::Fill),
            text(format!("{} snippets", commands.len()))
                .size(11)
                .font(FONT_MONO)
                .color(TEXT_MUTED),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    );

    let mut grid = column![].spacing(12);
    if commands.is_empty() {
        grid = grid.push(
            container(
                column![
                    icons::code(36.0, TEXT_MUTED),
                    text("No command snippets saved yet.")
                        .size(13)
                        .color(TEXT_SECONDARY),
                ]
                .spacing(8)
                .align_x(alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(32)
            .align_x(alignment::Horizontal::Center)
            .style(dark_card),
        );
    } else {
        for pair in commands.chunks(2) {
            let mut r = row![].spacing(12).width(Length::Fill);
            for item in pair {
                r = r.push(container(snippet_card(item)).width(Length::FillPortion(1)));
            }
            if pair.len() == 1 {
                r = r.push(space().width(Length::FillPortion(1)));
            }
            grid = grid.push(r);
        }
    }

    body = body.push(
        scrollable(grid)
            .height(Length::Fill)
            .style(dark_scrollable),
    );

    body.width(Length::Fill).height(Length::Fill).into()
}

fn snippet_card<'a>(item: &'a CommandSnippet) -> Element<'a, CommandsPageMsg> {
    let cmd_clone = item.cmd.clone();
    let id = item.id;

    let top = row![
        column![
            text(&item.name)
                .size(14)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            container(
                text(format!("{} uses", item.uses))
                    .size(10)
                    .font(FONT_MONO)
                    .color(BRAND_400),
            )
            .padding([2, 8])
            .style(pill_badge(
                rgba(BRAND_500, 0.14),
                rgba(BRAND_500, 0.3),
                BRAND_400,
            )),
        ]
        .spacing(4),
        space().width(Length::Fill),
        button(
            row![
                icons::play(12.0, GREEN_400),
                text("Run").size(11).font(FONT_MONO_BOLD).color(GREEN_400),
            ]
            .spacing(5)
            .align_y(alignment::Vertical::Center),
        )
        .padding([6, 10])
        .style(|_, status| {
            let alpha = match status {
                button::Status::Hovered | button::Status::Pressed => 0.24,
                _ => 0.12,
            };
            button::Style {
                background: Some(Background::Color(rgba(GREEN_500, alpha))),
                text_color: GREEN_400,
                border: Border {
                    color: rgba(GREEN_500, 0.35),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        })
        .on_press(CommandsPageMsg::Run(id, cmd_clone)),
        button(icons::trash(13.0, RED_400))
            .padding([6, 8])
            .style(danger_ghost_button)
            .on_press(CommandsPageMsg::Delete(id)),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    let code_box = container(
        text(format!("$ {}", item.cmd))
            .size(12)
            .font(FONT_MONO)
            .color(TEXT_LIGHT),
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

    container(column![top, code_box].spacing(10))
        .width(Length::Fill)
        .padding(14)
        .style(secondary_card_style)
        .into()
}

fn secondary_card_style(theme: &iced::Theme) -> container::Style {
    let mut st = dark_card(theme);
    st.background = Some(Background::Color(rgba(crate::theme::DARK_800, 0.9)));
    st
}

#[allow(dead_code)]
fn _unused(_: fn(&iced::Theme, button::Status) -> button::Style) {
    let _ = secondary_button;
}
