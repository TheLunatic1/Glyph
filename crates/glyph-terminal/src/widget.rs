use crate::emulator::{encode_iced_key, TerminalEmulator};
use crate::highlights::{compute_line_highlights, CompiledHighlightRule};
use crate::themes::TerminalThemeId;
use alacritty_terminal::index::Point;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, CursorShape, NamedColor};
use iced::keyboard::key::Named;
use iced::keyboard::{self, Key};
use iced::mouse;
use iced::widget::canvas::{self, Action, Event, Frame, Geometry, Path, Program, Text};
use iced::{alignment, Color, Font, Pixels, Point as IcedPoint, Rectangle, Renderer, Size, Theme};

#[derive(Debug, Clone)]
pub enum TerminalEvent {
    Input(Vec<u8>),
    Resized { cols: usize, rows: usize },
    CopyText(String),
    PasteRequested,
    ToggleSearch,
    FocusRequested,
    StartSelection { col: usize, row: usize },
    UpdateSelection { col: usize, row: usize },
    WheelScroll(i32),
}

pub struct TerminalCanvas<'a> {
    pub emulator: &'a TerminalEmulator,
    pub theme: TerminalThemeId,
    pub font_size: f32,
    pub font: Font,
    pub bold_font: Font,
    pub focused: bool,
    pub highlight_rules: &'a [CompiledHighlightRule],
    pub highlights_enabled: bool,
}

#[derive(Default)]
pub struct TerminalCanvasState {
    last_cols: usize,
    last_rows: usize,
    dragging: bool,
    drag_start: Option<(usize, usize)>,
}

impl<'a> TerminalCanvas<'a> {
    fn cell_metrics(&self) -> (f32, f32) {
        let w = (self.font_size * 0.602).max(4.0);
        let h = (self.font_size * 1.32).ceil().max(8.0);
        (w, h)
    }

    fn grid_for_bounds(&self, bounds: Rectangle) -> (usize, usize) {
        let (cw, ch) = self.cell_metrics();
        let pad = 8.0;
        let usable_w = (bounds.width - pad * 2.0).max(cw * 10.0);
        let usable_h = (bounds.height - pad * 2.0).max(ch * 4.0);
        let cols = ((usable_w / cw).floor() as usize).max(10);
        let rows = ((usable_h / ch).floor() as usize).max(4);
        (cols, rows)
    }

    fn pos_to_cell(&self, pos: IcedPoint) -> (usize, usize) {
        let (cw, ch) = self.cell_metrics();
        let pad = 8.0;
        let col = ((pos.x - pad).max(0.0) / cw).floor() as usize;
        let row = ((pos.y - pad).max(0.0) / ch).floor() as usize;
        (col, row)
    }
}

impl<'a> Program<TerminalEvent> for TerminalCanvas<'a> {
    type State = TerminalCanvasState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = self.theme.palette();
        let (cw, ch) = self.cell_metrics();
        let pad = 8.0;

        // Background fill
        frame.fill_rectangle(IcedPoint::ORIGIN, bounds.size(), palette.background);

        let row_highlights: Vec<Vec<Option<Color>>> =
            if self.highlights_enabled && !self.highlight_rules.is_empty() {
                self.emulator
                    .visible_lines()
                    .iter()
                    .map(|line| compute_line_highlights(line, self.highlight_rules))
                    .collect()
            } else {
                Vec::new()
            };

        let content = self.emulator.renderable();
        let offset = content.display_offset as i32;
        let visible_rows = self.emulator.rows() as i32;
        let visible_cols = self.emulator.cols() as i32;

        for indexed in content.display_iter {
            let point: Point = indexed.point;
            let row = point.line.0 + offset;
            if row < 0 || row >= visible_rows {
                continue;
            }
            let col = point.column.0 as i32;
            if col < 0 || col >= visible_cols {
                continue;
            }

            let cell = indexed.cell;
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }

            let cell_x = pad + col as f32 * cw;
            let cell_y = pad + row as f32 * ch;
            let cell_w = if cell.flags.contains(Flags::WIDE_CHAR) {
                cw * 2.0
            } else {
                cw
            };

            let bright = cell.flags.contains(Flags::BOLD);
            let mut fg = palette.resolve(cell.fg, content.colors, bright);
            if matches!(cell.fg, AnsiColor::Named(NamedColor::Foreground)) {
                if let Some(hl_color) = row_highlights
                    .get(row as usize)
                    .and_then(|cols| cols.get(col as usize))
                    .copied()
                    .flatten()
                {
                    fg = hl_color;
                }
            }
            let mut bg = palette.resolve(cell.bg, content.colors, false);

            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }

            let is_selected = content
                .selection
                .as_ref()
                .is_some_and(|sel| sel.contains_cell(&indexed, content.cursor.point, content.cursor.shape));

            let is_search_match = self.emulator.is_in_active_match(point);

            // Cell background (only draw when different from terminal bg or highlighted)
            if is_search_match {
                frame.fill_rectangle(
                    IcedPoint::new(cell_x, cell_y),
                    Size::new(cell_w, ch),
                    Color::from_rgba8(250, 204, 21, 0.38),
                );
            } else if is_selected {
                frame.fill_rectangle(
                    IcedPoint::new(cell_x, cell_y),
                    Size::new(cell_w, ch),
                    palette.selection,
                );
            } else if bg != palette.background {
                frame.fill_rectangle(
                    IcedPoint::new(cell_x, cell_y),
                    Size::new(cell_w, ch),
                    bg,
                );
            }

            if !cell.flags.contains(Flags::HIDDEN) && cell.c != ' ' && cell.c != '\0' {
                let mut buf = [0u8; 4];
                let ch_str = cell.c.encode_utf8(&mut buf);
                frame.fill_text(Text {
                    content: ch_str.to_string(),
                    position: IcedPoint::new(cell_x, cell_y + 1.0),
                    color: fg,
                    size: Pixels(self.font_size),
                    font: if cell.flags.contains(Flags::BOLD) {
                        self.bold_font
                    } else {
                        self.font
                    },
                    align_x: iced::alignment::Horizontal::Left.into(),
                    align_y: alignment::Vertical::Top,
                    ..Text::default()
                });
            }

            if cell.flags.intersects(Flags::ALL_UNDERLINES) {
                frame.fill_rectangle(
                    IcedPoint::new(cell_x, cell_y + ch - 2.0),
                    Size::new(cell_w, 1.0),
                    fg,
                );
            }
            if cell.flags.contains(Flags::STRIKEOUT) {
                frame.fill_rectangle(
                    IcedPoint::new(cell_x, cell_y + ch * 0.5),
                    Size::new(cell_w, 1.0),
                    fg,
                );
            }
        }

        // Cursor rendering
        if content.cursor.shape != CursorShape::Hidden {
            let cur_row = content.cursor.point.line.0 + offset;
            let cur_col = content.cursor.point.column.0 as i32;
            if cur_row >= 0 && cur_row < visible_rows && cur_col >= 0 && cur_col < visible_cols {
                let cx = pad + cur_col as f32 * cw;
                let cy = pad + cur_row as f32 * ch;
                if self.focused {
                    match content.cursor.shape {
                        CursorShape::Beam => {
                            frame.fill_rectangle(
                                IcedPoint::new(cx, cy),
                                Size::new(2.0, ch),
                                palette.cursor,
                            );
                        }
                        CursorShape::Underline => {
                            frame.fill_rectangle(
                                IcedPoint::new(cx, cy + ch - 2.0),
                                Size::new(cw, 2.0),
                                palette.cursor,
                            );
                        }
                        _ => {
                            let c_color = Color {
                                a: 0.72,
                                ..palette.cursor
                            };
                            frame.fill_rectangle(
                                IcedPoint::new(cx, cy),
                                Size::new(cw, ch),
                                c_color,
                            );
                        }
                    }
                } else {
                    frame.stroke(
                        &Path::rectangle(IcedPoint::new(cx, cy), Size::new(cw, ch)),
                        canvas::Stroke::default()
                            .with_color(palette.cursor)
                            .with_width(1.0),
                    );
                }
            }
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<TerminalEvent>> {
        // Detect container resize cleanly on layout/window ticks without swallowing mouse/key input
        if !matches!(event, Event::Mouse(_) | Event::Keyboard(_))
            && bounds.width > 24.0
            && bounds.height > 24.0
        {
            let (cols, rows) = self.grid_for_bounds(bounds);
            if cols != state.last_cols || rows != state.last_rows {
                state.last_cols = cols;
                state.last_rows = rows;
                return Some(Action::publish(TerminalEvent::Resized { cols, rows }));
            }
        }

        match event {
            Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if let Some(pos) = cursor.position_in(bounds) {
                        state.dragging = true;
                        let (col, row) = self.pos_to_cell(pos);
                        state.drag_start = Some((col, row));
                        return Some(
                            Action::publish(TerminalEvent::StartSelection { col, row })
                                .and_capture(),
                        );
                    }
                }
                mouse::Event::CursorMoved { .. } if state.dragging => {
                    if let Some(pos) = cursor.position_in(bounds) {
                        let (col, row) = self.pos_to_cell(pos);
                        if state.drag_start != Some((col, row)) {
                            state.drag_start = Some((col, row));
                            return Some(
                                Action::publish(TerminalEvent::UpdateSelection { col, row })
                                    .and_capture(),
                            );
                        }
                    }
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    state.dragging = false;
                }
                mouse::Event::ButtonPressed(mouse::Button::Right) => {
                    if cursor.is_over(bounds) {
                        return Some(
                            Action::publish(TerminalEvent::PasteRequested).and_capture(),
                        );
                    }
                }
                mouse::Event::WheelScrolled { delta } if cursor.is_over(bounds) => {
                    let lines = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => (*y * 3.0).round() as i32,
                        mouse::ScrollDelta::Pixels { y, .. } => (*y / 16.0).round() as i32,
                    };
                    if lines != 0 {
                        return Some(
                            Action::publish(TerminalEvent::WheelScroll(lines)).and_capture(),
                        );
                    }
                }
                _ => {}
            },
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modified_key: _,
                modifiers,
                text,
                ..
            }) if self.focused => {
                // Ctrl+F / Cmd+F opens search bar
                if (modifiers.control() || modifiers.logo())
                    && matches!(key, Key::Character(c) if c.eq_ignore_ascii_case("f"))
                {
                    return Some(Action::publish(TerminalEvent::ToggleSearch).and_capture());
                }

                // Ctrl+Shift+C or Ctrl+C with active selection copies
                if (modifiers.control() || modifiers.logo())
                    && matches!(key, Key::Character(c) if c.eq_ignore_ascii_case("c"))
                {
                    if let Some(sel) = self.emulator.selected_text() {
                        return Some(Action::publish(TerminalEvent::CopyText(sel)).and_capture());
                    }
                }

                // Ctrl+V / Ctrl+Shift+V / Cmd+V pastes
                if (modifiers.control() || modifiers.logo())
                    && matches!(key, Key::Character(c) if c.eq_ignore_ascii_case("v"))
                {
                    return Some(Action::publish(TerminalEvent::PasteRequested).and_capture());
                }

                // Shift+PageUp / PageDown local scrollback can pass through or let shell handle
                if modifiers.shift() && matches!(key, Key::Named(Named::PageUp | Named::PageDown)) {
                    return None;
                }

                if let Some(bytes) = encode_iced_key(
                    key,
                    *modifiers,
                    text.as_deref(),
                    self.emulator.app_cursor_mode(),
                ) {
                    return Some(Action::publish(TerminalEvent::Input(bytes)).and_capture());
                }
            }
            _ => {}
        }

        None
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }
}
