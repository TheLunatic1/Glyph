use crate::theme::{
    brand_button, danger_ghost_button, dark_card, dark_input, dark_scrollable, glass_panel,
    pill_badge, rgba, secondary_button, truncate_ellipsis, BRAND_300, BRAND_400, BRAND_500,
    DARK_700, DARK_900, FONT_MONO, FONT_UI_BOLD, FONT_UI_SEMIBOLD, GREEN_400, GREEN_500, RED_400,
    TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons;
use glyph_core::models::{CommandSnippet, SnippetPackage};
use glyph_core::vault::extract_template_vars;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{alignment, Background, Border, Color, Element, Length};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandsSubView {
    #[default]
    ServerSnippets,
    Packages,
}

#[derive(Debug, Clone)]
pub enum CommandsPageMsg {
    SwitchSubView(CommandsSubView),
    NameInput(String),
    CmdInput(String),
    AddSubmit,
    Run(u64, String, String),
    RunPackageSnippet(String, String),
    RunOnGroup(String, String),
    Delete(u64),
    SelectPackage(String),
    ToggleNewPackageForm,
    NewPackageNameInput(String),
    NewPackageDescInput(String),
    AddPackageSubmit,
    DeletePackage(String),
    AddPackageSnippetSubmit,
    DeletePackageSnippet(String, u64),
}

#[derive(Debug, Clone, Default)]
pub struct CommandsPageState {
    pub sub_view: CommandsSubView,
    pub selected_package_id: String,
    pub show_new_pkg: bool,
    pub new_pkg_name: String,
    pub new_pkg_desc: String,
    pub new_name: String,
    pub new_cmd: String,
    pub error: Option<String>,
}

pub fn view<'a>(
    st: &'a CommandsPageState,
    commands: &'a [CommandSnippet],
    packages: &'a [SnippetPackage],
) -> Element<'a, CommandsPageMsg> {
    let sub_tabs = row![
        button(
            text("This Server")
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(if st.sub_view == CommandsSubView::ServerSnippets {
                    Color::WHITE
                } else {
                    TEXT_SECONDARY
                }),
        )
        .padding([7, 12])
        .style(if st.sub_view == CommandsSubView::ServerSnippets {
            brand_button
        } else {
            secondary_button
        })
        .on_press(CommandsPageMsg::SwitchSubView(CommandsSubView::ServerSnippets)),
        button(
            text(format!("Global Packages ({})", packages.len()))
                .size(12)
                .font(FONT_UI_SEMIBOLD)
                .color(if st.sub_view == CommandsSubView::Packages {
                    Color::WHITE
                } else {
                    TEXT_SECONDARY
                }),
        )
        .padding([7, 12])
        .style(if st.sub_view == CommandsSubView::Packages {
            brand_button
        } else {
            secondary_button
        })
        .on_press(CommandsPageMsg::SwitchSubView(CommandsSubView::Packages)),
    ]
    .spacing(8);

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
                snap: true,
                ..Default::default()
            }),
        column![
            text("Saved Commands & Snippet Packages")
                .size(20)
                .font(FONT_UI_BOLD)
                .color(TEXT_PRIMARY),
            text("Run snippets on this session, substitute {{vars}}, or execute across a host group")
                .size(12)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        sub_tabs,
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let mut body = column![header].spacing(16).padding(24);

    if let Some(err) = &st.error {
        body = body.push(
            container(text(err).size(12).color(RED_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
        );
    }

    match st.sub_view {
        CommandsSubView::ServerSnippets => {
            let add_form = container(
                column![
                    text("Add Server Command Snippet")
                        .size(13)
                        .font(FONT_UI_SEMIBOLD)
                        .color(TEXT_LIGHT),
                    row![
                        text_input("Snippet Name (e.g. Reload Nginx)", &st.new_name)
                            .on_input(CommandsPageMsg::NameInput)
                            .on_submit(CommandsPageMsg::AddSubmit)
                            .padding([9, 12])
                            .size(13)
                            .width(Length::FillPortion(2))
                            .style(dark_input),
                        text_input("Shell Command (supports {{var}} templates)", &st.new_cmd)
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
                                text("Add")
                                    .size(13)
                                    .font(FONT_UI_SEMIBOLD)
                                    .color(Color::WHITE),
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

            body = body.push(add_form).push(
                row![
                    icons::zap(15.0, BRAND_400),
                    text("Sorted by Most Used")
                        .size(13)
                        .font(FONT_UI_SEMIBOLD)
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
                        let sid = item.id;
                        let sname = item.name.clone();
                        let scmd = item.cmd.clone();
                        r = r.push(container(snippet_card(
                            item,
                            CommandsPageMsg::Run(sid, sname.clone(), scmd.clone()),
                            CommandsPageMsg::RunOnGroup(sname, scmd),
                            CommandsPageMsg::Delete(sid),
                        ))
                        .width(Length::FillPortion(1)));
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
        }
        CommandsSubView::Packages => {
            let active_pkg = packages
                .iter()
                .find(|p| p.id == st.selected_package_id)
                .or_else(|| packages.first());

            let mut pkg_pills = row![].spacing(8).align_y(alignment::Vertical::Center);
            for pkg in packages {
                let selected = active_pkg.map(|a| a.id == pkg.id).unwrap_or(false);
                pkg_pills = pkg_pills.push(
                    button(
                        row![
                            icons::command(12.0, if selected { Color::WHITE } else { BRAND_400 }),
                            text(format!("{} ({})", pkg.name, pkg.snippets.len()))
                                .size(12)
                                .font(FONT_UI_SEMIBOLD)
                                .color(if selected { Color::WHITE } else { TEXT_LIGHT }),
                        ]
                        .spacing(6)
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding([7, 12])
                    .style(if selected {
                        brand_button
                    } else {
                        secondary_button
                    })
                    .on_press(CommandsPageMsg::SelectPackage(pkg.id.clone())),
                );
            }
            pkg_pills = pkg_pills.push(
                button(
                    row![
                        icons::plus(13.0, BRAND_300),
                        text("New Package").size(12).color(BRAND_300),
                    ]
                    .spacing(5)
                    .align_y(alignment::Vertical::Center),
                )
                .padding([7, 11])
                .style(secondary_button)
                .on_press(CommandsPageMsg::ToggleNewPackageForm),
            );

            body = body.push(
                scrollable(pkg_pills)
                    .horizontal()
                    .width(Length::Fill)
                    .style(dark_scrollable),
            );

            if st.show_new_pkg {
                body = body.push(
                    container(
                        row![
                            text_input("Package Name (e.g. Kubernetes)", &st.new_pkg_name)
                                .on_input(CommandsPageMsg::NewPackageNameInput)
                                .padding([8, 10])
                                .size(12)
                                .width(Length::FillPortion(2))
                                .style(dark_input),
                            text_input("Description", &st.new_pkg_desc)
                                .on_input(CommandsPageMsg::NewPackageDescInput)
                                .on_submit(CommandsPageMsg::AddPackageSubmit)
                                .padding([8, 10])
                                .size(12)
                                .width(Length::FillPortion(3))
                                .style(dark_input),
                            button(text("Create").size(12).color(Color::WHITE))
                                .padding([8, 14])
                                .style(brand_button)
                                .on_press(CommandsPageMsg::AddPackageSubmit),
                        ]
                        .spacing(8)
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding(12)
                    .width(Length::Fill)
                    .style(glass_panel),
                );
            }

            if let Some(pkg) = active_pkg {
                let pkg_header = row![
                    column![
                        text(&pkg.name)
                            .size(15)
                            .font(FONT_UI_BOLD)
                            .color(TEXT_PRIMARY),
                        text(&pkg.description).size(12).color(TEXT_SECONDARY),
                    ]
                    .spacing(2),
                    space().width(Length::Fill),
                    button(icons::trash(14.0, RED_400))
                        .padding([6, 10])
                        .style(danger_ghost_button)
                        .on_press(CommandsPageMsg::DeletePackage(pkg.id.clone())),
                ]
                .align_y(alignment::Vertical::Center);

                let add_pkg_snip = row![
                    text_input("Snippet Name", &st.new_name)
                        .on_input(CommandsPageMsg::NameInput)
                        .padding([8, 10])
                        .size(12)
                        .width(Length::FillPortion(2))
                        .style(dark_input),
                    text_input("Shell Command (supports {{var}} templates)", &st.new_cmd)
                        .on_input(CommandsPageMsg::CmdInput)
                        .on_submit(CommandsPageMsg::AddPackageSnippetSubmit)
                        .padding([8, 10])
                        .size(12)
                        .font(FONT_MONO)
                        .width(Length::FillPortion(3))
                        .style(dark_input),
                    button(text("Add to Package").size(12).color(Color::WHITE))
                        .padding([8, 14])
                        .style(brand_button)
                        .on_press(CommandsPageMsg::AddPackageSnippetSubmit),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center);

                body = body.push(
                    container(column![pkg_header, add_pkg_snip].spacing(10))
                        .padding(14)
                        .width(Length::Fill)
                        .style(glass_panel),
                );

                let mut grid = column![].spacing(12);
                if pkg.snippets.is_empty() {
                    grid = grid.push(
                        container(
                            text("This package has no snippets yet.")
                                .size(13)
                                .color(TEXT_SECONDARY),
                        )
                        .padding(24)
                        .width(Length::Fill)
                        .align_x(alignment::Horizontal::Center)
                        .style(dark_card),
                    );
                } else {
                    for pair in pkg.snippets.chunks(2) {
                        let mut r = row![].spacing(12).width(Length::Fill);
                        for item in pair {
                            let sid = item.id;
                            let sname = item.name.clone();
                            let scmd = item.cmd.clone();
                            r = r.push(container(snippet_card(
                                item,
                                CommandsPageMsg::RunPackageSnippet(sname.clone(), scmd.clone()),
                                CommandsPageMsg::RunOnGroup(sname, scmd),
                                CommandsPageMsg::DeletePackageSnippet(pkg.id.clone(), sid),
                            ))
                            .width(Length::FillPortion(1)));
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
            }
        }
    }

    body.width(Length::Fill).height(Length::Fill).into()
}

fn snippet_card<'a>(
    item: &'a CommandSnippet,
    on_run: CommandsPageMsg,
    on_group: CommandsPageMsg,
    on_delete: CommandsPageMsg,
) -> Element<'a, CommandsPageMsg> {
    let vars = extract_template_vars(&item.cmd);
    let mut badges = row![
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
    .spacing(6);

    if !vars.is_empty() {
        badges = badges.push(
            container(
                text(format!("{{{{{}}}}}", vars.join(", ")))
                    .size(10)
                    .font(FONT_MONO)
                    .color(BRAND_300),
            )
            .padding([2, 8])
            .style(pill_badge(
                rgba(BRAND_400, 0.14),
                rgba(BRAND_400, 0.35),
                BRAND_300,
            )),
        );
    }

    let top = row![
        column![
            text(truncate_ellipsis(&item.name, 28))
                .size(14)
                .font(FONT_UI_BOLD)
                .color(TEXT_PRIMARY),
            badges,
        ]
        .spacing(4),
        space().width(Length::Fill),
        button(
            row![
                icons::users(12.0, BRAND_300),
                text("Group").size(11).color(BRAND_300),
            ]
            .spacing(4)
            .align_y(alignment::Vertical::Center),
        )
        .padding([6, 9])
        .style(secondary_button)
        .on_press(on_group),
        button(
            row![
                icons::play(12.0, GREEN_400),
                text("Run").size(11).font(FONT_UI_SEMIBOLD).color(GREEN_400),
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
                snap: true,
                ..Default::default()
            }
        })
        .on_press(on_run),
        button(icons::trash(13.0, RED_400))
            .padding([6, 8])
            .style(danger_ghost_button)
            .on_press(on_delete),
    ]
    .spacing(6)
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
        snap: true,
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
