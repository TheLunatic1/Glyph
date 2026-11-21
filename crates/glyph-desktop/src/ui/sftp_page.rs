use crate::theme::{
    brand_button, danger_button, danger_ghost_button, dark_card, dark_input, dark_scrollable,
    dark_text_editor, ghost_icon_button, glass_panel, modal_card, pill_badge, rgba,
    secondary_button, truncate_ellipsis, BRAND_300, BRAND_400, BRAND_500, DARK_600, DARK_700,
    DARK_800, DARK_900, FONT_MONO, FONT_MONO_BOLD, GREEN_400, GREEN_500, RED_400, RED_500,
    TEXT_DIM, TEXT_LIGHT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400,
};
use crate::ui::icons;
use glyph_core::sftp::{format_permissions_octal, join_remote_path, SftpEntry, SftpProgress, TransferKind};
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, space, stack, text, text_editor,
    text_input,
};
use iced::{alignment, Background, Border, Color, Element, Length};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SftpSortCol {
    Name,
    Permissions,
    Size,
}

#[derive(Debug, Clone)]
pub struct SftpClipboard {
    pub path: String,
    pub name: String,
    pub is_copy: bool,
}

#[derive(Debug, Clone)]
pub enum SftpPrompt {
    NewFolder,
    NewFile,
    Rename { old_path: String },
    Chmod { path: String },
    ConfirmDelete { path: String, name: String, is_dir: bool },
}

pub struct SftpEditorState {
    pub path: String,
    pub content: text_editor::Content,
    pub dirty: bool,
    pub saving: bool,
}

pub struct SftpPageState {
    pub opened: bool,
    pub current_path: String,
    pub entries: Vec<SftpEntry>,
    pub sort_col: SftpSortCol,
    pub sort_asc: bool,
    pub loading: bool,
    pub drag_hover: bool,
    pub copied_path: bool,
    pub clipboard: Option<SftpClipboard>,
    pub prompt: Option<SftpPrompt>,
    pub prompt_input: String,
    pub editor: Option<SftpEditorState>,
    pub transfer: Option<SftpProgress>,
    pub status: Option<String>,
    pub error: Option<String>,
}

impl Default for SftpPageState {
    fn default() -> Self {
        Self {
            opened: false,
            current_path: "/".to_string(),
            entries: Vec::new(),
            sort_col: SftpSortCol::Name,
            sort_asc: true,
            loading: false,
            drag_hover: false,
            copied_path: false,
            clipboard: None,
            prompt: None,
            prompt_input: String::new(),
            editor: None,
            transfer: None,
            status: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SftpPageMsg {
    OpenRoot,
    Navigate(String),
    GoUp,
    Refresh,
    CopyCurrentPath,
    ToggleSort(SftpSortCol),

    OpenEntry(String, bool),
    DownloadFile(String, String),
    UploadPickFiles,

    CopyItem(SftpEntry),
    CutItem(SftpEntry),
    PasteClipboard,

    OpenNewFolderPrompt,
    OpenNewFilePrompt,
    OpenRenamePrompt(String, String),
    OpenChmodPrompt(String, u32),
    OpenDeletePrompt(String, String, bool),
    PromptInputChanged(String),
    PromptSubmit,
    PromptCancel,

    EditorAction(text_editor::Action),
    EditorSave,
    EditorClose,
    DismissTransfer,
}

pub fn view<'a>(st: &'a SftpPageState) -> Element<'a, SftpPageMsg> {
    if !st.opened {
        return view_landing(st.loading, st.error.as_deref());
    }

    if let Some(editor) = &st.editor {
        return view_editor(editor, st.error.as_deref(), st.status.as_deref());
    }

    let up_btn = button(icons::arrow_up(15.0, TEXT_LIGHT))
        .padding([7, 10])
        .style(secondary_button)
        .on_press(SftpPageMsg::GoUp);

    let breadcrumb_bar = build_breadcrumbs(&st.current_path, st.copied_path);

    let mut actions = row![].spacing(6).align_y(alignment::Vertical::Center);
    if st.clipboard.is_some() {
        actions = actions.push(
            button(
                row![
                    icons::clipboard(13.0, BRAND_300),
                    text("Paste").size(12).color(BRAND_300),
                ]
                .spacing(5)
                .align_y(alignment::Vertical::Center),
            )
            .padding([7, 10])
            .style(secondary_button)
            .on_press(SftpPageMsg::PasteClipboard),
        );
    }

    actions = actions
        .push(
            button(
                row![
                    icons::folder_plus(14.0, TEXT_LIGHT),
                    text("Folder").size(12).color(TEXT_LIGHT),
                ]
                .spacing(5)
                .align_y(alignment::Vertical::Center),
            )
            .padding([7, 10])
            .style(secondary_button)
            .on_press(SftpPageMsg::OpenNewFolderPrompt),
        )
        .push(
            button(
                row![
                    icons::file_plus(14.0, TEXT_LIGHT),
                    text("File").size(12).color(TEXT_LIGHT),
                ]
                .spacing(5)
                .align_y(alignment::Vertical::Center),
            )
            .padding([7, 10])
            .style(secondary_button)
            .on_press(SftpPageMsg::OpenNewFilePrompt),
        )
        .push(
            button(
                row![
                    icons::upload(14.0, Color::WHITE),
                    text("Upload").size(12).font(FONT_MONO_BOLD).color(Color::WHITE),
                ]
                .spacing(5)
                .align_y(alignment::Vertical::Center),
            )
            .padding([7, 12])
            .style(brand_button)
            .on_press(SftpPageMsg::UploadPickFiles),
        )
        .push(
            button(icons::refresh_cw(14.0, TEXT_LIGHT))
                .padding([7, 10])
                .style(secondary_button)
                .on_press(SftpPageMsg::Refresh),
        );

    let top_bar = container(
        column![
            row![up_btn, breadcrumb_bar]
                .spacing(10)
                .align_y(alignment::Vertical::Center),
            row![space().width(Length::Fill), actions]
                .align_y(alignment::Vertical::Center),
        ]
        .spacing(8),
    )
    .width(Length::Fill)
    .padding(12)
    .style(glass_panel);

    let mut content = column![top_bar].spacing(12).padding(20);

    if let Some(prompt) = &st.prompt {
        content = content.push(view_prompt_bar(prompt, &st.prompt_input));
    }

    if let Some(err) = &st.error {
        content = content.push(
            container(
                row![
                    icons::alert_triangle(14.0, RED_400),
                    text(err).size(12).color(RED_400),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([8, 12])
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
        );
    }

    if let Some(msg) = &st.status {
        content = content.push(
            container(
                row![
                    icons::check(14.0, GREEN_400),
                    text(msg).size(12).color(GREEN_400),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([8, 12])
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(GREEN_500, 0.12))),
                border: Border {
                    color: rgba(GREEN_500, 0.35),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                snap: true,
                ..Default::default()
            }),
        );
    }

    let table_head = container(
        row![
            sort_header_btn("Name", SftpSortCol::Name, st.sort_col, st.sort_asc, Length::FillPortion(5)),
            sort_header_btn("Permissions", SftpSortCol::Permissions, st.sort_col, st.sort_asc, Length::Fixed(105.0)),
            sort_header_btn("Size", SftpSortCol::Size, st.sort_col, st.sort_asc, Length::Fixed(95.0)),
            text("Actions")
                .size(11)
                .font(FONT_MONO_BOLD)
                .color(TEXT_MUTED)
                .width(Length::Fixed(190.0)),
        ]
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding([8, 12])
    .style(|_| container::Style {
        background: Some(Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 8.0.into(),
        },
        snap: true,
        ..Default::default()
    });

    let mut sorted: Vec<&SftpEntry> = st.entries.iter().collect();
    sorted.sort_by(|a, b| cmp_entries(a, b, st.sort_col, st.sort_asc));

    let mut list_col = column![].spacing(4);
    if st.loading {
        list_col = list_col.push(
            container(text("Loading directory listing...").size(12).color(TEXT_MUTED))
                .width(Length::Fill)
                .padding(24)
                .align_x(alignment::Horizontal::Center),
        );
    } else if sorted.is_empty() {
        list_col = list_col.push(
            container(
                column![
                    icons::folder_open(32.0, TEXT_MUTED),
                    text("This directory is empty")
                        .size(13)
                        .color(TEXT_SECONDARY),
                    text("Drag & drop local files here to upload via SFTP")
                        .size(11)
                        .color(TEXT_MUTED),
                ]
                .spacing(6)
                .align_x(alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(32)
            .align_x(alignment::Horizontal::Center),
        );
    } else {
        for entry in sorted {
            list_col = list_col.push(view_entry_row(entry));
        }
    }

    let table_box = container(
        column![
            table_head,
            scrollable(list_col)
                .height(Length::Fill)
                .style(dark_scrollable),
        ]
        .spacing(8),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(12)
    .style(|_| container::Style {
        background: Some(Background::Color(DARK_800)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 12.0.into(),
        },
        snap: true,
        ..Default::default()
    });

    content = content.push(table_box);

    if let Some(tr) = &st.transfer {
        content = content.push(view_transfer_toast(tr));
    }

    let base: Element<'a, SftpPageMsg> = content.width(Length::Fill).height(Length::Fill).into();

    if st.drag_hover {
        let overlay = container(
            container(
                column![
                    icons::upload(40.0, BRAND_300),
                    text(format!("Drop files to upload to {}", st.current_path))
                        .size(16)
                        .font(FONT_MONO_BOLD)
                        .color(TEXT_PRIMARY),
                ]
                .spacing(10)
                .align_x(alignment::Horizontal::Center),
            )
            .padding(32)
            .style(modal_card),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(rgba(BRAND_500, 0.18))),
            border: Border {
                color: BRAND_400,
                width: 2.0,
                radius: 12.0.into(),
            },
            snap: true,
            ..Default::default()
        });
        stack![base, overlay]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        base
    }
}

fn view_landing<'a>(loading: bool, error: Option<&'a str>) -> Element<'a, SftpPageMsg> {
    let mut inner = column![
        container(icons::folder_open(42.0, BRAND_400))
            .padding(18)
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(BRAND_500, 0.14))),
                border: Border {
                    color: rgba(BRAND_500, 0.35),
                    width: 1.0,
                    radius: 16.0.into(),
                },
                snap: true,
                ..Default::default()
            }),
        text("File Explorer")
            .size(22)
            .font(FONT_MONO_BOLD)
            .color(TEXT_PRIMARY),
        text("Click to open root directory over SFTP")
            .size(13)
            .color(TEXT_SECONDARY),
        button(
            row![
                icons::folder(15.0, Color::WHITE),
                text(if loading { "Opening..." } else { "Browse /" })
                    .size(13)
                    .font(FONT_MONO_BOLD)
                    .color(Color::WHITE),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .padding([10, 22])
        .style(brand_button)
        .on_press(SftpPageMsg::OpenRoot),
    ]
    .spacing(12)
    .align_x(alignment::Horizontal::Center);

    if let Some(err) = error {
        inner = inner.push(text(err).size(12).color(RED_400));
    }

    container(
        container(inner)
            .padding(36)
            .style(glass_panel),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn build_breadcrumbs<'a>(current_path: &'a str, copied: bool) -> Element<'a, SftpPageMsg> {
    let mut crumbs = row![button(
        text("/")
            .size(12)
            .font(FONT_MONO_BOLD)
            .color(BRAND_300),
    )
    .padding([4, 8])
    .style(secondary_button)
    .on_press(SftpPageMsg::Navigate("/".to_string()))]
    .spacing(4)
    .align_y(alignment::Vertical::Center);

    let mut acc = String::new();
    for seg in current_path.split('/').filter(|s| !s.is_empty()) {
        acc = join_remote_path(&acc, seg);
        let target = acc.clone();
        crumbs = crumbs
            .push(text("/").size(12).color(TEXT_MUTED))
            .push(
                button(text(seg).size(12).font(FONT_MONO).color(TEXT_LIGHT))
                    .padding([4, 7])
                    .style(ghost_icon_button)
                    .on_press(SftpPageMsg::Navigate(target)),
            );
    }

    container(
        row![
            container(
                scrollable(crumbs)
                    .horizontal()
                    .width(Length::Fill)
                    .style(dark_scrollable),
            )
            .width(Length::Fill)
            .clip(true),
            button(
                row![
                    if copied {
                        icons::check(12.0, GREEN_400)
                    } else {
                        icons::copy(12.0, TEXT_SECONDARY)
                    },
                    text(if copied { "Copied" } else { "Copy" })
                        .size(11)
                        .color(if copied { GREEN_400 } else { TEXT_SECONDARY })
                        .wrapping(text::Wrapping::None),
                ]
                .spacing(4)
                .align_y(alignment::Vertical::Center),
            )
            .padding([4, 8])
            .style(ghost_icon_button)
            .on_press(SftpPageMsg::CopyCurrentPath),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding([4, 10])
    .style(|_| container::Style {
        background: Some(Background::Color(DARK_900)),
        border: Border {
            color: DARK_700,
            width: 1.0,
            radius: 8.0.into(),
        },
        snap: true,
        ..Default::default()
    })
    .into()
}

fn view_prompt_bar<'a>(prompt: &'a SftpPrompt, input: &'a str) -> Element<'a, SftpPageMsg> {
    match prompt {
        SftpPrompt::ConfirmDelete { name, is_dir, .. } => container(
            row![
                icons::alert_triangle(16.0, RED_400),
                text(format!(
                    "Delete {} \"{}\" permanently?",
                    if *is_dir { "directory" } else { "file" },
                    name
                ))
                .size(13)
                .color(TEXT_PRIMARY),
                space().width(Length::Fill),
                button(text("Cancel").size(12).color(TEXT_LIGHT))
                    .padding([6, 12])
                    .style(secondary_button)
                    .on_press(SftpPageMsg::PromptCancel),
                button(text("Delete").size(12).font(FONT_MONO_BOLD).color(Color::WHITE))
                    .padding([6, 14])
                    .style(danger_button)
                    .on_press(SftpPageMsg::PromptSubmit),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding([10, 14])
        .style(modal_card)
        .into(),
        _ => {
            let (label, placeholder) = match prompt {
                SftpPrompt::NewFolder => ("New Folder Name:", "e.g. backups"),
                SftpPrompt::NewFile => ("New File Name:", "e.g. .env"),
                SftpPrompt::Rename { .. } => ("Rename To:", "New filename"),
                SftpPrompt::Chmod { .. } => ("Octal Mode (chmod):", "e.g. 755 or 644"),
                SftpPrompt::ConfirmDelete { .. } => unreachable!(),
            };
            container(
                row![
                    text(label).size(12).font(FONT_MONO_BOLD).color(TEXT_LIGHT),
                    text_input(placeholder, input)
                        .on_input(SftpPageMsg::PromptInputChanged)
                        .on_submit(SftpPageMsg::PromptSubmit)
                        .padding([7, 10])
                        .size(12)
                        .font(FONT_MONO)
                        .width(Length::Fill)
                        .style(dark_input),
                    button(text("Cancel").size(12).color(TEXT_LIGHT))
                        .padding([6, 12])
                        .style(secondary_button)
                        .on_press(SftpPageMsg::PromptCancel),
                    button(text("Apply").size(12).font(FONT_MONO_BOLD).color(Color::WHITE))
                        .padding([6, 14])
                        .style(brand_button)
                        .on_press(SftpPageMsg::PromptSubmit),
                ]
                .spacing(10)
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([10, 14])
            .style(modal_card)
            .into()
        }
    }
}

fn sort_header_btn<'a>(
    label: &'static str,
    col: SftpSortCol,
    active: SftpSortCol,
    asc: bool,
    width: Length,
) -> Element<'a, SftpPageMsg> {
    let mut r = row![text(label)
        .size(11)
        .font(FONT_MONO_BOLD)
        .color(if active == col { BRAND_300 } else { TEXT_MUTED })]
    .spacing(4)
    .align_y(alignment::Vertical::Center);
    if active == col {
        r = r.push(if asc {
            icons::arrow_up(11.0, BRAND_300)
        } else {
            icons::arrow_down(11.0, BRAND_300)
        });
    }
    button(r)
        .padding([2, 4])
        .width(width)
        .style(|_, _| button::Style {
            background: None,
            snap: true,
            ..Default::default()
        })
        .on_press(SftpPageMsg::ToggleSort(col))
        .into()
}

fn view_entry_row<'a>(entry: &'a SftpEntry) -> Element<'a, SftpPageMsg> {
    let path = entry.path.clone();
    let name = entry.filename.clone();
    let is_dir = entry.is_dir;
    let octal = format_permissions_octal(entry.permissions);

    let icon = if is_dir {
        icons::folder(16.0, BRAND_400)
    } else {
        icons::file_text(16.0, TEXT_SECONDARY)
    };

    let mut name_row = row![
        icon,
        container(
            text(truncate_ellipsis(&entry.filename, 36))
                .size(12)
                .font(FONT_MONO)
                .color(if is_dir { TEXT_PRIMARY } else { TEXT_LIGHT })
                .wrapping(text::Wrapping::None),
        )
        .width(Length::Fill)
        .clip(true),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    if entry.is_symlink {
        name_row = name_row.push(
            container(text("link").size(9).color(YELLOW_400))
                .padding([1, 5])
                .style(pill_badge(
                    rgba(YELLOW_400, 0.12),
                    rgba(YELLOW_400, 0.3),
                    YELLOW_400,
                )),
        );
    }

    let name_btn = button(name_row)
        .padding([4, 6])
        .width(Length::FillPortion(5))
        .clip(true)
        .style(|_, status| {
            let bg = match status {
                button::Status::Hovered => rgba(BRAND_500, 0.08),
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                snap: true,
                ..Default::default()
            }
        })
        .on_press(SftpPageMsg::OpenEntry(path.clone(), is_dir));

    let perms_btn = button(
        container(
            text(octal)
                .size(11)
                .font(FONT_MONO)
                .color(TEXT_SECONDARY),
        )
        .padding([2, 7])
        .style(|_| container::Style {
            background: Some(Background::Color(DARK_800)),
            border: Border {
                color: DARK_700,
                width: 1.0,
                radius: 6.0.into(),
            },
            snap: true,
            ..Default::default()
        }),
    )
    .width(Length::Fixed(105.0))
    .padding(0)
    .style(|_, _| button::Style {
        background: None,
        snap: true,
        ..Default::default()
    })
    .on_press(SftpPageMsg::OpenChmodPrompt(path.clone(), entry.permissions));

    let size_str = if is_dir {
        "--".to_string()
    } else {
        format_bytes(entry.size)
    };

    let mut act_row = row![].spacing(4).width(Length::Fixed(190.0));
    if !is_dir {
        act_row = act_row
            .push(
                button(icons::edit(13.0, BRAND_400))
                    .padding(5)
                    .style(ghost_icon_button)
                    .on_press(SftpPageMsg::OpenEntry(path.clone(), false)),
            )
            .push(
                button(icons::download(13.0, GREEN_400))
                    .padding(5)
                    .style(ghost_icon_button)
                    .on_press(SftpPageMsg::DownloadFile(path.clone(), name.clone())),
            );
    }
    act_row = act_row
        .push(
            button(icons::copy(13.0, TEXT_SECONDARY))
                .padding(5)
                .style(ghost_icon_button)
                .on_press(SftpPageMsg::CopyItem(entry.clone())),
        )
        .push(
            button(icons::scissors(13.0, TEXT_SECONDARY))
                .padding(5)
                .style(ghost_icon_button)
                .on_press(SftpPageMsg::CutItem(entry.clone())),
        )
        .push(
            button(icons::sliders(13.0, TEXT_SECONDARY))
                .padding(5)
                .style(ghost_icon_button)
                .on_press(SftpPageMsg::OpenRenamePrompt(path.clone(), name.clone())),
        )
        .push(
            button(icons::trash(13.0, RED_400))
                .padding(5)
                .style(danger_ghost_button)
                .on_press(SftpPageMsg::OpenDeletePrompt(path, name, is_dir)),
        );

    mouse_area(
        container(
            row![
                name_btn,
                perms_btn,
                text(size_str)
                    .size(11)
                    .font(FONT_MONO)
                    .color(TEXT_MUTED)
                    .width(Length::Fixed(95.0)),
                act_row,
            ]
            .align_y(alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding([6, 10])
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
    )
    .into()
}

fn view_editor<'a>(
    ed: &'a SftpEditorState,
    error: Option<&'a str>,
    status: Option<&'a str>,
) -> Element<'a, SftpPageMsg> {
    let save_btn = button(
        row![
            icons::save(14.0, Color::WHITE),
            text(if ed.saving { "Saving..." } else { "Save" })
                .size(12)
                .font(FONT_MONO_BOLD)
                .color(Color::WHITE),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding([7, 14])
    .style(brand_button);
    let save_btn = if ed.saving {
        save_btn
    } else {
        save_btn.on_press(SftpPageMsg::EditorSave)
    };

    let header = container(
        row![
            button(
                row![
                    icons::arrow_left(14.0, TEXT_LIGHT),
                    text("Back").size(12).color(TEXT_LIGHT),
                ]
                .spacing(5)
                .align_y(alignment::Vertical::Center),
            )
            .padding([7, 12])
            .style(secondary_button)
            .on_press(SftpPageMsg::EditorClose),
            icons::file_text(16.0, BRAND_400),
            text(&ed.path)
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            if ed.dirty {
                container(text("Modified").size(10).color(YELLOW_400))
                    .padding([2, 8])
                    .style(pill_badge(
                        rgba(YELLOW_400, 0.14),
                        rgba(YELLOW_400, 0.35),
                        YELLOW_400,
                    ))
            } else {
                container(text("Saved").size(10).color(GREEN_400))
                    .padding([2, 8])
                    .style(pill_badge(
                        rgba(GREEN_500, 0.14),
                        rgba(GREEN_500, 0.35),
                        GREEN_400,
                    ))
            },
            space().width(Length::Fill),
            save_btn,
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding(12)
    .style(glass_panel);

    let mut col = column![header].spacing(12).padding(20);

    if let Some(err) = error {
        col = col.push(
            container(text(err).size(12).color(RED_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
        );
    }
    if let Some(msg) = status {
        col = col.push(
            container(text(msg).size(12).color(GREEN_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(dark_card),
        );
    }

    let editor_widget = text_editor(&ed.content)
        .font(FONT_MONO)
        .size(13.0)
        .height(Length::Fill)
        .padding(14)
        .style(dark_text_editor)
        .on_action(SftpPageMsg::EditorAction);

    col = col.push(
        container(editor_widget)
            .width(Length::Fill)
            .height(Length::Fill),
    );

    col.width(Length::Fill).height(Length::Fill).into()
}

fn view_transfer_toast<'a>(tr: &'a SftpProgress) -> Element<'a, SftpPageMsg> {
    let pct = tr
        .transferred
        .saturating_mul(100)
        .checked_div(tr.total)
        .unwrap_or(100)
        .min(100) as u16;
    let label = match tr.kind {
        TransferKind::Upload => "Uploading",
        TransferKind::Download => "Downloading",
    };

    let fill_p = pct.max(1);
    let rem_p = 100u16.saturating_sub(fill_p).max(if fill_p == 100 { 0 } else { 1 });

    let mut bar_row = row![container(space().width(Length::Fill).height(6.0))
        .width(Length::FillPortion(fill_p))
        .style(|_| container::Style {
            background: Some(Background::Color(BRAND_500)),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            snap: true,
            ..Default::default()
        })];
    if rem_p > 0 {
        bar_row = bar_row.push(space().width(Length::FillPortion(rem_p)));
    }

    container(
        column![
            row![
                text(format!("{label} {}", tr.filename))
                    .size(12)
                    .font(FONT_MONO_BOLD)
                    .color(TEXT_PRIMARY),
                space().width(Length::Fill),
                text(format!(
                    "{pct}%  ({} / {})",
                    format_bytes(tr.transferred),
                    format_bytes(tr.total)
                ))
                .size(11)
                .font(FONT_MONO)
                .color(TEXT_SECONDARY),
                button(icons::x_close(12.0, TEXT_MUTED))
                    .padding(3)
                    .style(ghost_icon_button)
                    .on_press(SftpPageMsg::DismissTransfer),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
            container(bar_row)
                .width(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(DARK_700)),
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    snap: true,
                    ..Default::default()
                }),
        ]
        .spacing(6),
    )
    .width(Length::Fill)
    .padding(12)
    .style(modal_card)
    .into()
}

fn cmp_entries(a: &SftpEntry, b: &SftpEntry, col: SftpSortCol, asc: bool) -> std::cmp::Ordering {
    match (a.is_dir, b.is_dir) {
        (true, false) => return std::cmp::Ordering::Less,
        (false, true) => return std::cmp::Ordering::Greater,
        _ => {}
    }
    let ord = match col {
        SftpSortCol::Name => a
            .filename
            .to_ascii_lowercase()
            .cmp(&b.filename.to_ascii_lowercase()),
        SftpSortCol::Permissions => a.permissions.cmp(&b.permissions),
        SftpSortCol::Size => a.size.cmp(&b.size),
    };
    if asc {
        ord
    } else {
        ord.reverse()
    }
}

fn format_bytes(b: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let f = b as f64;
    if f >= GB {
        format!("{:.1} GB", f / GB)
    } else if f >= MB {
        format!("{:.1} MB", f / MB)
    } else if f >= KB {
        format!("{:.1} KB", f / KB)
    } else {
        format!("{b} B")
    }
}

#[allow(dead_code)]
fn _keep() {
    let _ = (DARK_600, TEXT_DIM);
}
