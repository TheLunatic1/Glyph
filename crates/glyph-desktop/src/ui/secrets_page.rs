use crate::theme::{
    brand_button, danger_ghost_button, dark_card, dark_input, dark_scrollable, glass_panel,
    ghost_icon_button, modal_card, rgba, secondary_button, BRAND_400, BRAND_500, DARK_700,
    DARK_800, FONT_MONO, FONT_MONO_BOLD, GREEN_400, GREEN_500, RED_400, RED_500, TEXT_LIGHT,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, YELLOW_400, YELLOW_500,
};
use crate::ui::icons;
use glyph_core::models::SecretSummary;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{alignment, Background, Border, Color, Element, Length};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretsBundleMode {
    Export,
    Import,
}

#[derive(Debug, Clone, Default)]
pub struct SecretsPageState {
    pub new_name: String,
    pub new_value: String,
    pub show_value: bool,
    pub copied_id: Option<String>,
    pub status: Option<String>,
    pub error: Option<String>,

    pub bundle_modal: Option<SecretsBundleMode>,
    pub bundle_password: String,
    pub show_bundle_password: bool,
}

#[derive(Debug, Clone)]
pub enum SecretsPageMsg {
    NameInput(String),
    ValueInput(String),
    ToggleShowValue,
    AddSubmit,
    CopyValue(String),
    Delete(String),
    OpenExportModal,
    OpenImportModal,
    CloseBundleModal,
    BundlePasswordInput(String),
    ToggleBundlePassword,
    BundleSubmit,
}

pub fn view<'a>(
    st: &'a SecretsPageState,
    secrets: &'a [SecretSummary],
    encryption_available: bool,
) -> Element<'a, SecretsPageMsg> {
    let header = row![
        container(icons::lock(20.0, BRAND_400))
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
            text("Secrets Vault")
                .size(20)
                .font(FONT_MONO_BOLD)
                .color(TEXT_PRIMARY),
            text("Encrypted per-server API tokens, passwords, and credentials")
                .size(12)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
        space().width(Length::Fill),
        button(
            row![
                icons::download(14.0, TEXT_LIGHT),
                text("Export").size(12).color(TEXT_LIGHT),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 12])
        .style(secondary_button)
        .on_press(SecretsPageMsg::OpenExportModal),
        button(
            row![
                icons::upload(14.0, TEXT_LIGHT),
                text("Import").size(12).color(TEXT_LIGHT),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding([8, 12])
        .style(secondary_button)
        .on_press(SecretsPageMsg::OpenImportModal),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);

    let mut body = column![header].spacing(16).padding(24);

    if !encryption_available {
        body = body.push(
            container(
                row![
                    icons::alert_triangle(15.0, YELLOW_400),
                    text("OS keychain unavailable — secrets are secured with machine-wrapped encryption.")
                        .size(12)
                        .color(YELLOW_400),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .padding([10, 12])
            .style(|_| container::Style {
                background: Some(Background::Color(rgba(YELLOW_500, 0.12))),
                border: Border {
                    color: rgba(YELLOW_500, 0.35),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }),
        );
    }

    let add_form = container(
        column![
            text("Store New Secret")
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(TEXT_LIGHT),
            row![
                text_input("Secret Alias (e.g. DB_PASSWORD)", &st.new_name)
                    .on_input(SecretsPageMsg::NameInput)
                    .padding([9, 12])
                    .size(13)
                    .font(FONT_MONO)
                    .width(Length::FillPortion(2))
                    .style(dark_input),
                text_input("Secret Value", &st.new_value)
                    .secure(!st.show_value)
                    .on_input(SecretsPageMsg::ValueInput)
                    .on_submit(SecretsPageMsg::AddSubmit)
                    .padding([9, 12])
                    .size(13)
                    .font(FONT_MONO)
                    .width(Length::FillPortion(3))
                    .style(dark_input),
                button(if st.show_value {
                    icons::eye_off(15.0, TEXT_SECONDARY)
                } else {
                    icons::eye(15.0, TEXT_SECONDARY)
                })
                .padding([9, 10])
                .style(secondary_button)
                .on_press(SecretsPageMsg::ToggleShowValue),
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
                .on_press(SecretsPageMsg::AddSubmit),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(10),
    )
    .width(Length::Fill)
    .padding(16)
    .style(glass_panel);

    body = body.push(add_form);

    if let Some(mode) = st.bundle_modal {
        let title = match mode {
            SecretsBundleMode::Export => "Export Encrypted .glyph Secrets Bundle",
            SecretsBundleMode::Import => "Import Encrypted .glyph Secrets Bundle",
        };
        let action_label = match mode {
            SecretsBundleMode::Export => "Choose Save Location",
            SecretsBundleMode::Import => "Choose Backup File",
        };

        body = body.push(
            container(
                column![
                    row![
                        text(title)
                            .size(13)
                            .font(FONT_MONO_BOLD)
                            .color(TEXT_PRIMARY),
                        space().width(Length::Fill),
                        button(icons::x_close(14.0, TEXT_SECONDARY))
                            .padding(4)
                            .style(ghost_icon_button)
                            .on_press(SecretsPageMsg::CloseBundleModal),
                    ]
                    .align_y(alignment::Vertical::Center),
                    row![
                        text_input("Master Password (AES-256-GCM + scrypt)", &st.bundle_password)
                            .secure(!st.show_bundle_password)
                            .on_input(SecretsPageMsg::BundlePasswordInput)
                            .on_submit(SecretsPageMsg::BundleSubmit)
                            .padding([8, 12])
                            .size(13)
                            .width(Length::Fill)
                            .style(dark_input),
                        button(if st.show_bundle_password {
                            icons::eye_off(15.0, TEXT_SECONDARY)
                        } else {
                            icons::eye(15.0, TEXT_SECONDARY)
                        })
                        .padding([8, 10])
                        .style(secondary_button)
                        .on_press(SecretsPageMsg::ToggleBundlePassword),
                        button(
                            text(action_label)
                                .size(12)
                                .font(FONT_MONO_BOLD)
                                .color(Color::WHITE),
                        )
                        .padding([8, 14])
                        .style(brand_button)
                        .on_press(SecretsPageMsg::BundleSubmit),
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                ]
                .spacing(10),
            )
            .width(Length::Fill)
            .padding(14)
            .style(modal_card),
        );
    }

    if let Some(err) = &st.error {
        body = body.push(
            container(text(err).size(12).color(RED_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(|_| container::Style {
                    background: Some(Background::Color(rgba(RED_500, 0.12))),
                    border: Border {
                        color: rgba(RED_500, 0.35),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
        );
    }
    if let Some(msg) = &st.status {
        body = body.push(
            container(text(msg).size(12).color(GREEN_400))
                .width(Length::Fill)
                .padding([8, 12])
                .style(|_| container::Style {
                    background: Some(Background::Color(rgba(GREEN_500, 0.12))),
                    border: Border {
                        color: rgba(GREEN_500, 0.35),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
        );
    }

    body = body.push(
        row![
            icons::shield_check(15.0, GREEN_400),
            text(format!("Encrypted Keys ({})", secrets.len()))
                .size(13)
                .font(FONT_MONO_BOLD)
                .color(TEXT_LIGHT),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    );

    let mut grid = column![].spacing(12);
    if secrets.is_empty() {
        grid = grid.push(
            container(
                column![
                    icons::lock(36.0, TEXT_MUTED),
                    text("No secrets stored for this server yet.")
                        .size(13)
                        .color(TEXT_SECONDARY),
                    text("Use the Secrets menu in Terminal to inject values directly without echoing.")
                        .size(11)
                        .color(TEXT_MUTED),
                ]
                .spacing(6)
                .align_x(alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(32)
            .align_x(alignment::Horizontal::Center)
            .style(dark_card),
        );
    } else {
        for pair in secrets.chunks(2) {
            let mut r = row![].spacing(12).width(Length::Fill);
            for item in pair {
                let is_copied = st.copied_id.as_deref() == Some(item.id.as_str());
                let sid = item.id.clone();
                let card = container(
                    row![
                        container(icons::key(15.0, BRAND_400))
                            .padding(8)
                            .style(|_| container::Style {
                                background: Some(Background::Color(rgba(BRAND_500, 0.12))),
                                border: Border {
                                    color: rgba(BRAND_500, 0.25),
                                    width: 1.0,
                                    radius: 8.0.into(),
                                },
                                ..Default::default()
                            }),
                        column![
                            text(&item.name)
                                .size(13)
                                .font(FONT_MONO_BOLD)
                                .color(TEXT_PRIMARY),
                            text("Encrypted via OS Keychain")
                                .size(11)
                                .color(TEXT_MUTED),
                        ]
                        .spacing(2),
                        space().width(Length::Fill),
                        button(
                            row![
                                if is_copied {
                                    icons::check(13.0, GREEN_400)
                                } else {
                                    icons::copy(13.0, TEXT_SECONDARY)
                                },
                                text(if is_copied { "Copied" } else { "Copy" })
                                    .size(11)
                                    .color(if is_copied { GREEN_400 } else { TEXT_LIGHT }),
                            ]
                            .spacing(5)
                            .align_y(alignment::Vertical::Center),
                        )
                        .padding([6, 10])
                        .style(secondary_button)
                        .on_press(SecretsPageMsg::CopyValue(sid.clone())),
                        button(icons::trash(13.0, RED_400))
                            .padding([6, 8])
                            .style(danger_ghost_button)
                            .on_press(SecretsPageMsg::Delete(sid)),
                    ]
                    .spacing(10)
                    .align_y(alignment::Vertical::Center),
                )
                .width(Length::FillPortion(1))
                .padding(14)
                .style(|theme| {
                    let mut c = dark_card(theme);
                    c.background = Some(Background::Color(rgba(DARK_800, 0.9)));
                    c
                });
                r = r.push(card);
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

    let _ = DARK_700;
    body.width(Length::Fill).height(Length::Fill).into()
}
