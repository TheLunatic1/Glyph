pub mod emulator;
pub mod themes;
pub mod widget;

pub use emulator::{encode_iced_key, TerminalEmulator};
pub use themes::{TerminalPalette, TerminalThemeId};
pub use widget::{TerminalCanvas, TerminalEvent};
