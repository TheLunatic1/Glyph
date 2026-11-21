use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};
use iced::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalThemeId {
    #[default]
    GlyphDefault,
    Dracula,
    OneDark,
    SolarizedDark,
    Monokai,
}

impl TerminalThemeId {
    pub const ALL: [Self; 5] = [
        Self::GlyphDefault,
        Self::Dracula,
        Self::OneDark,
        Self::SolarizedDark,
        Self::Monokai,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::GlyphDefault => "Glyph Default",
            Self::Dracula => "Dracula",
            Self::OneDark => "One Dark",
            Self::SolarizedDark => "Solarized Dark",
            Self::Monokai => "Monokai",
        }
    }

    pub fn palette(&self) -> TerminalPalette {
        match self {
            Self::GlyphDefault => TerminalPalette {
                background: hex(0x0f111a, 1.0),
                foreground: hex(0xf3f4f6, 1.0),
                cursor: hex(0x6366f1, 1.0),
                selection: hex(0x818cf8, 0.31),
                ansi: [
                    hex(0x1a1d2d, 1.0), // black
                    hex(0xf87171, 1.0), // red
                    hex(0x4ade80, 1.0), // green
                    hex(0xfacc15, 1.0), // yellow
                    hex(0x6366f1, 1.0), // blue
                    hex(0xc084fc, 1.0), // magenta
                    hex(0x22d3ee, 1.0), // cyan
                    hex(0xe5e7eb, 1.0), // white
                    hex(0x4b5563, 1.0), // bright black
                    hex(0xfca5a5, 1.0), // bright red
                    hex(0x86efac, 1.0), // bright green
                    hex(0xfde047, 1.0), // bright yellow
                    hex(0x818cf8, 1.0), // bright blue
                    hex(0xd8b4fe, 1.0), // bright magenta
                    hex(0x67e8f9, 1.0), // bright cyan
                    hex(0xffffff, 1.0), // bright white
                ],
            },
            Self::Dracula => TerminalPalette {
                background: hex(0x282a36, 1.0),
                foreground: hex(0xf8f8f2, 1.0),
                cursor: hex(0xff79c6, 1.0),
                selection: hex(0x44475a, 0.85),
                ansi: [
                    hex(0x21222c, 1.0),
                    hex(0xff5555, 1.0),
                    hex(0x50fa7b, 1.0),
                    hex(0xf1fa8c, 1.0),
                    hex(0xbd93f9, 1.0),
                    hex(0xff79c6, 1.0),
                    hex(0x8be9fd, 1.0),
                    hex(0xf8f8f2, 1.0),
                    hex(0x6272a4, 1.0),
                    hex(0xff6e6e, 1.0),
                    hex(0x69ff94, 1.0),
                    hex(0xffffa5, 1.0),
                    hex(0xd6acff, 1.0),
                    hex(0xff92df, 1.0),
                    hex(0xa4ffff, 1.0),
                    hex(0xffffff, 1.0),
                ],
            },
            Self::OneDark => TerminalPalette {
                background: hex(0x282c34, 1.0),
                foreground: hex(0xabb2bf, 1.0),
                cursor: hex(0x528bff, 1.0),
                selection: hex(0x3e4451, 0.85),
                ansi: [
                    hex(0x1e2127, 1.0),
                    hex(0xe06c75, 1.0),
                    hex(0x98c379, 1.0),
                    hex(0xd19a66, 1.0),
                    hex(0x61afef, 1.0),
                    hex(0xc678dd, 1.0),
                    hex(0x56b6c2, 1.0),
                    hex(0xabb2bf, 1.0),
                    hex(0x5c6370, 1.0),
                    hex(0xe06c75, 1.0),
                    hex(0x98c379, 1.0),
                    hex(0xe5c07b, 1.0),
                    hex(0x61afef, 1.0),
                    hex(0xc678dd, 1.0),
                    hex(0x56b6c2, 1.0),
                    hex(0xffffff, 1.0),
                ],
            },
            Self::SolarizedDark => TerminalPalette {
                background: hex(0x002b36, 1.0),
                foreground: hex(0x839496, 1.0),
                cursor: hex(0x93a1a1, 1.0),
                selection: hex(0x073642, 0.90),
                ansi: [
                    hex(0x073642, 1.0),
                    hex(0xdc322f, 1.0),
                    hex(0x859900, 1.0),
                    hex(0xb58900, 1.0),
                    hex(0x268bd2, 1.0),
                    hex(0xd33682, 1.0),
                    hex(0x2aa198, 1.0),
                    hex(0xeee8d5, 1.0),
                    hex(0x002b36, 1.0),
                    hex(0xcb4b16, 1.0),
                    hex(0x586e75, 1.0),
                    hex(0x657b83, 1.0),
                    hex(0x839496, 1.0),
                    hex(0x6c71c4, 1.0),
                    hex(0x93a1a1, 1.0),
                    hex(0xfdf6e3, 1.0),
                ],
            },
            Self::Monokai => TerminalPalette {
                background: hex(0x272822, 1.0),
                foreground: hex(0xf8f8f2, 1.0),
                cursor: hex(0xf8f8f0, 1.0),
                selection: hex(0x49483e, 0.85),
                ansi: [
                    hex(0x272822, 1.0),
                    hex(0xf92672, 1.0),
                    hex(0xa6e22e, 1.0),
                    hex(0xf4bf75, 1.0),
                    hex(0x66d9ef, 1.0),
                    hex(0xae81ff, 1.0),
                    hex(0xa1efe4, 1.0),
                    hex(0xf8f8f2, 1.0),
                    hex(0x75715e, 1.0),
                    hex(0xf92672, 1.0),
                    hex(0xa6e22e, 1.0),
                    hex(0xf4bf75, 1.0),
                    hex(0x66d9ef, 1.0),
                    hex(0xae81ff, 1.0),
                    hex(0xa1efe4, 1.0),
                    hex(0xf9f8f5, 1.0),
                ],
            },
        }
    }
}

impl std::fmt::Display for TerminalThemeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalPalette {
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    pub selection: Color,
    pub ansi: [Color; 16],
}

impl TerminalPalette {
    pub fn resolve(&self, color: AnsiColor, dynamic_colors: &Colors, bright: bool) -> Color {
        match color {
            AnsiColor::Spec(rgb) => Color::from_rgb8(rgb.r, rgb.g, rgb.b),
            AnsiColor::Named(named) => {
                if let Some(rgb) = dynamic_colors[named] {
                    return Color::from_rgb8(rgb.r, rgb.g, rgb.b);
                }
                match named {
                    NamedColor::Black => {
                        if bright {
                            self.ansi[8]
                        } else {
                            self.ansi[0]
                        }
                    }
                    NamedColor::Red => {
                        if bright {
                            self.ansi[9]
                        } else {
                            self.ansi[1]
                        }
                    }
                    NamedColor::Green => {
                        if bright {
                            self.ansi[10]
                        } else {
                            self.ansi[2]
                        }
                    }
                    NamedColor::Yellow => {
                        if bright {
                            self.ansi[11]
                        } else {
                            self.ansi[3]
                        }
                    }
                    NamedColor::Blue => {
                        if bright {
                            self.ansi[12]
                        } else {
                            self.ansi[4]
                        }
                    }
                    NamedColor::Magenta => {
                        if bright {
                            self.ansi[13]
                        } else {
                            self.ansi[5]
                        }
                    }
                    NamedColor::Cyan => {
                        if bright {
                            self.ansi[14]
                        } else {
                            self.ansi[6]
                        }
                    }
                    NamedColor::White => {
                        if bright {
                            self.ansi[15]
                        } else {
                            self.ansi[7]
                        }
                    }
                    NamedColor::BrightBlack => self.ansi[8],
                    NamedColor::BrightRed => self.ansi[9],
                    NamedColor::BrightGreen => self.ansi[10],
                    NamedColor::BrightYellow => self.ansi[11],
                    NamedColor::BrightBlue => self.ansi[12],
                    NamedColor::BrightMagenta => self.ansi[13],
                    NamedColor::BrightCyan => self.ansi[14],
                    NamedColor::BrightWhite => self.ansi[15],
                    NamedColor::Foreground | NamedColor::BrightForeground => self.foreground,
                    NamedColor::Background => self.background,
                    NamedColor::Cursor => self.cursor,
                    NamedColor::DimBlack => dim(self.ansi[0]),
                    NamedColor::DimRed => dim(self.ansi[1]),
                    NamedColor::DimGreen => dim(self.ansi[2]),
                    NamedColor::DimYellow => dim(self.ansi[3]),
                    NamedColor::DimBlue => dim(self.ansi[4]),
                    NamedColor::DimMagenta => dim(self.ansi[5]),
                    NamedColor::DimCyan => dim(self.ansi[6]),
                    NamedColor::DimWhite => dim(self.ansi[7]),
                    NamedColor::DimForeground => dim(self.foreground),
                }
            }
            AnsiColor::Indexed(idx) => {
                if let Some(rgb) = dynamic_colors[idx as usize] {
                    return Color::from_rgb8(rgb.r, rgb.g, rgb.b);
                }
                match idx {
                    0..=15 => self.ansi[idx as usize],
                    16..=231 => {
                        let c = idx - 16;
                        let r = c / 36;
                        let g = (c % 36) / 6;
                        let b = c % 6;
                        let scale = |v: u8| if v == 0 { 0 } else { v * 40 + 55 };
                        Color::from_rgb8(scale(r), scale(g), scale(b))
                    }
                    232..=255 => {
                        let v = (idx - 232) * 10 + 8;
                        Color::from_rgb8(v, v, v)
                    }
                }
            }
        }
    }
}

const fn hex(rgb: u32, a: f32) -> Color {
    let r = ((rgb >> 16) & 0xff) as f32 / 255.0;
    let g = ((rgb >> 8) & 0xff) as f32 / 255.0;
    let b = (rgb & 0xff) as f32 / 255.0;
    Color { r, g, b, a }
}

fn dim(c: Color) -> Color {
    Color {
        r: c.r * 0.66,
        g: c.g * 0.66,
        b: c.b * 0.66,
        a: c.a,
    }
}
