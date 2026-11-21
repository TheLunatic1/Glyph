pub mod emulator;
pub mod highlights;
pub mod recorder;
pub mod themes;
pub mod widget;

pub use emulator::{encode_iced_key, TerminalEmulator};
pub use highlights::{compile_highlight_rules, compute_line_highlights, CompiledHighlightRule};
pub use recorder::{default_logs_dir, export_plain_log, SessionRecorder};
pub use themes::{TerminalPalette, TerminalThemeId};
pub use widget::{TerminalCanvas, TerminalEvent};
