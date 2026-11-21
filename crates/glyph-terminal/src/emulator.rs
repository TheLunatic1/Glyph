use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Scroll;
use alacritty_terminal::index::{Column, Direction, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::search::{Match, RegexSearch};
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, RenderableContent, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};

pub struct TerminalEmulator {
    pub term: Term<VoidListener>,
    processor: Processor,
    cols: usize,
    rows: usize,
    pub search_query: String,
    pub active_match: Option<Match>,
}

impl TerminalEmulator {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(10);
        let rows = rows.max(4);
        let size = TermSize::new(cols, rows);
        let config = Config {
            scrolling_history: 10_000,
            ..Config::default()
        };
        let term = Term::new(config, &size, VoidListener);
        Self {
            term,
            processor: Processor::new(),
            cols,
            rows,
            search_query: String::new(),
            active_match: None,
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn feed_bytes(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.term, bytes);
    }

    pub fn resize(&mut self, cols: usize, rows: usize) -> bool {
        let cols = cols.max(10);
        let rows = rows.max(4);
        if cols == self.cols && rows == self.rows {
            return false;
        }
        self.cols = cols;
        self.rows = rows;
        let size = TermSize::new(cols, rows);
        self.term.resize(size);
        true
    }

    pub fn clear(&mut self) {
        self.processor.advance(&mut self.term, b"\x1b[3J\x1b[2J\x1b[H");
        self.term.selection = None;
    }

    pub fn scroll_display(&mut self, lines: i32) {
        if lines != 0 {
            self.term.scroll_display(Scroll::Delta(lines));
        }
    }

    pub fn start_selection(&mut self, viewport_col: usize, viewport_row: usize) {
        let point = self.viewport_point(viewport_col, viewport_row);
        self.term.selection = Some(Selection::new(SelectionType::Simple, point, Side::Left));
    }

    pub fn update_selection(&mut self, viewport_col: usize, viewport_row: usize) {
        let point = self.viewport_point(viewport_col, viewport_row);
        if let Some(sel) = &mut self.term.selection {
            sel.update(point, Side::Right);
        }
    }

    pub fn clear_selection(&mut self) {
        self.term.selection = None;
    }

    pub fn selected_text(&self) -> Option<String> {
        self.term.selection_to_string().filter(|s| !s.is_empty())
    }

    fn viewport_point(&self, col: usize, row: usize) -> Point {
        let offset = self.term.grid().display_offset();
        let c = Column(col.min(self.cols.saturating_sub(1)));
        let r = row.min(self.rows.saturating_sub(1));
        alacritty_terminal::term::viewport_to_point(offset, Point::new(r, c))
    }

    pub fn app_cursor_mode(&self) -> bool {
        self.term.mode().contains(TermMode::APP_CURSOR)
    }

    pub fn renderable(&self) -> RenderableContent<'_> {
        self.term.renderable_content()
    }

    pub fn search(&mut self, query: &str, forward: bool) -> bool {
        self.search_query = query.to_string();
        if query.is_empty() {
            self.active_match = None;
            return false;
        }
        let escaped = escape_regex_literal(query);
        let Ok(mut dfas) = RegexSearch::new(&escaped) else {
            self.active_match = None;
            return false;
        };

        let origin = self
            .active_match
            .as_ref()
            .map(|m| {
                if forward {
                    let next_col = m.start().column + 1;
                    if next_col.0 < self.cols {
                        Point::new(m.start().line, next_col)
                    } else {
                        Point::new(m.start().line + 1, Column(0))
                    }
                } else if m.start().column.0 > 0 {
                    Point::new(m.start().line, m.start().column - 1)
                } else {
                    Point::new(m.start().line - 1, Column(self.cols.saturating_sub(1)))
                }
            })
            .unwrap_or_else(|| Point::new(Line(0), Column(0)));

        let dir = if forward {
            Direction::Right
        } else {
            Direction::Left
        };

        let found = self
            .term
            .search_next(&mut dfas, origin, dir, Side::Left, None);

        if let Some(ref m) = found {
            self.term.scroll_to_point(*m.start());
        }
        self.active_match = found.clone();
        found.is_some()
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.active_match = None;
    }

    pub fn is_in_active_match(&self, point: Point) -> bool {
        self.active_match
            .as_ref()
            .is_some_and(|m| point >= *m.start() && point <= *m.end())
    }

    /// Reads visible screen text lines (helpful for tests and accessibility).
    pub fn visible_lines(&self) -> Vec<String> {
        let mut lines = vec![String::new(); self.rows];
        let content = self.term.renderable_content();
        let offset = content.display_offset as i32;
        for indexed in content.display_iter {
            if indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            let row = indexed.point.line.0 + offset;
            if row >= 0 && (row as usize) < self.rows {
                lines[row as usize].push(indexed.cell.c);
            }
        }
        lines
            .into_iter()
            .map(|l| l.trim_end().to_string())
            .collect()
    }
}

fn escape_regex_literal(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for ch in input.chars() {
        if matches!(
            ch,
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Converts an Iced key event into xterm/VT100 wire bytes.
pub fn encode_iced_key(
    key: &Key,
    modifiers: Modifiers,
    text: Option<&str>,
    app_cursor: bool,
) -> Option<Vec<u8>> {
    let ctrl = modifiers.control();
    let alt = modifiers.alt();
    let shift = modifiers.shift();

    if let Key::Named(named) = key {
        let bytes: Option<Vec<u8>> = match named {
            Named::Enter => Some(vec![b'\r']),
            Named::Backspace => Some(if alt {
                vec![0x1b, 0x7f]
            } else {
                vec![0x7f]
            }),
            Named::Tab => Some(if shift {
                b"\x1b[Z".to_vec()
            } else {
                vec![b'\t']
            }),
            Named::Escape => Some(vec![0x1b]),
            Named::Space => {
                if ctrl {
                    Some(vec![0x00])
                } else if alt {
                    Some(vec![0x1b, b' '])
                } else {
                    Some(vec![b' '])
                }
            }
            Named::ArrowUp => Some(cursor_seq(b'A', modifiers, app_cursor)),
            Named::ArrowDown => Some(cursor_seq(b'B', modifiers, app_cursor)),
            Named::ArrowRight => Some(cursor_seq(b'C', modifiers, app_cursor)),
            Named::ArrowLeft => Some(cursor_seq(b'D', modifiers, app_cursor)),
            Named::Home => Some(if app_cursor {
                b"\x1bOH".to_vec()
            } else {
                b"\x1b[H".to_vec()
            }),
            Named::End => Some(if app_cursor {
                b"\x1bOF".to_vec()
            } else {
                b"\x1b[F".to_vec()
            }),
            Named::Insert => Some(b"\x1b[2~".to_vec()),
            Named::Delete => Some(b"\x1b[3~".to_vec()),
            Named::PageUp => Some(b"\x1b[5~".to_vec()),
            Named::PageDown => Some(b"\x1b[6~".to_vec()),
            Named::F1 => Some(b"\x1bOP".to_vec()),
            Named::F2 => Some(b"\x1bOQ".to_vec()),
            Named::F3 => Some(b"\x1bOR".to_vec()),
            Named::F4 => Some(b"\x1bOS".to_vec()),
            Named::F5 => Some(b"\x1b[15~".to_vec()),
            Named::F6 => Some(b"\x1b[17~".to_vec()),
            Named::F7 => Some(b"\x1b[18~".to_vec()),
            Named::F8 => Some(b"\x1b[19~".to_vec()),
            Named::F9 => Some(b"\x1b[20~".to_vec()),
            Named::F10 => Some(b"\x1b[21~".to_vec()),
            Named::F11 => Some(b"\x1b[23~".to_vec()),
            Named::F12 => Some(b"\x1b[24~".to_vec()),
            _ => None,
        };
        if bytes.is_some() {
            return bytes;
        }
    }

    if let Key::Character(c_str) = key {
        let mut ch_iter = c_str.chars();
        if let Some(ch) = ch_iter.next() {
            if ctrl {
                let lower = ch.to_ascii_lowercase();
                let code = match lower {
                    'a'..='z' => Some((lower as u8) - b'a' + 1),
                    '[' => Some(0x1b),
                    '\\' => Some(0x1c),
                    ']' => Some(0x1d),
                    '^' | '6' => Some(0x1e),
                    '_' | '-' => Some(0x1f),
                    '@' | '2' => Some(0x00),
                    _ => None,
                };
                if let Some(byte) = code {
                    return Some(if alt { vec![0x1b, byte] } else { vec![byte] });
                }
            }
        }
    }

    if let Some(txt) = text {
        if !txt.is_empty() && !ctrl {
            let mut out = Vec::new();
            if alt {
                out.push(0x1b);
            }
            out.extend_from_slice(txt.as_bytes());
            return Some(out);
        }
    }

    if let Key::Character(c_str) = key {
        if !ctrl {
            let mut out = Vec::new();
            if alt {
                out.push(0x1b);
            }
            out.extend_from_slice(c_str.as_bytes());
            return Some(out);
        }
    }

    None
}

fn cursor_seq(dir: u8, mods: Modifiers, app_cursor: bool) -> Vec<u8> {
    let mod_code = 1
        + u8::from(mods.shift())
        + (u8::from(mods.alt()) * 2)
        + (u8::from(mods.control()) * 4);
    if mod_code > 1 {
        format!("\x1b[1;{mod_code}{}", dir as char).into_bytes()
    } else if app_cursor {
        vec![0x1b, b'O', dir]
    } else {
        vec![0x1b, b'[', dir]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi_parsing_and_visible_lines() {
        let mut emu = TerminalEmulator::new(40, 10);
        emu.feed_bytes(b"\x1b[32mHello Glyph\x1b[0m\r\nSecond line");
        let lines = emu.visible_lines();
        assert_eq!(lines[0], "Hello Glyph");
        assert_eq!(lines[1], "Second line");
    }

    #[test]
    fn resize_and_search_work() {
        let mut emu = TerminalEmulator::new(40, 5);
        emu.feed_bytes(b"line1 alpha\r\nline2 beta\r\nline3 alpha\r\n");
        assert!(emu.search("beta", true));
        assert!(emu.active_match.is_some());
        assert!(emu.resize(80, 24));
        assert_eq!(emu.cols(), 80);
        assert_eq!(emu.rows(), 24);
    }

    #[test]
    fn encodes_ctrl_c_and_arrows() {
        assert_eq!(
            encode_iced_key(
                &Key::Character("c".into()),
                Modifiers::CTRL,
                None,
                false
            ),
            Some(vec![0x03])
        );
        assert_eq!(
            encode_iced_key(&Key::Named(Named::ArrowUp), Modifiers::empty(), None, false),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            encode_iced_key(&Key::Named(Named::ArrowUp), Modifiers::empty(), None, true),
            Some(b"\x1bOA".to_vec())
        );
    }
}
