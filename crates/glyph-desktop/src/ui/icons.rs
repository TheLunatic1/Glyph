use iced::widget::text;
use iced::{alignment, Color, Font};
use lucide_icons::Icon;

pub const LUCIDE_FONT_BYTES: &[u8] = lucide_icons::LUCIDE_FONT_BYTES;

pub const FONT_LUCIDE: Font = Font {
    family: iced::font::Family::Name("lucide"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

fn icon<'a>(glyph: Icon, size: f32, color: Color) -> text::Text<'a> {
    text(char::from(glyph).to_string())
        .font(FONT_LUCIDE)
        .size(size.round().max(10.0))
        .line_height(1.0)
        .color(color)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
}

/// Renders an `X` close icon without pinning color so parent button hover states can tint it.
pub fn x_close_inherit<'a>(size: f32) -> text::Text<'a> {
    text(char::from(Icon::X).to_string())
        .font(FONT_LUCIDE)
        .size(size.round().max(10.0))
        .line_height(1.0)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
}

pub fn activity<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Activity, size, color)
}

pub fn terminal<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Terminal, size, color)
}

pub fn folder_open<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::FolderOpen, size, color)
}

pub fn code<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::CodeXml, size, color)
}

pub fn lock<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Lock, size, color)
}

pub fn network<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Network, size, color)
}

pub fn cube<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Box, size, color)
}

pub fn monitor<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Monitor, size, color)
}

pub fn log_out<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::LogOut, size, color)
}

pub fn smartphone<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Smartphone, size, color)
}

pub fn qr_code<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::QrCode, size, color)
}

pub fn plus<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Plus, size, color)
}

pub fn minus<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Minus, size, color)
}

pub fn play<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Play, size, color)
}

pub fn trash<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Trash2, size, color)
}

pub fn edit<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Pencil, size, color)
}

pub fn shield_check<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::ShieldCheck, size, color)
}

pub fn download<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Download, size, color)
}

pub fn upload<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Upload, size, color)
}

pub fn settings<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Settings, size, color)
}

pub fn key<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::KeyRound, size, color)
}

pub fn server<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Server, size, color)
}

pub fn x_close<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::X, size, color)
}

pub fn eye<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Eye, size, color)
}

pub fn eye_off<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::EyeOff, size, color)
}

pub fn cpu<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Cpu, size, color)
}

pub fn memory_stick<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::MemoryStick, size, color)
}

pub fn hard_drive<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::HardDrive, size, color)
}

pub fn wifi<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Wifi, size, color)
}

pub fn circuit_board<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::CircuitBoard, size, color)
}

pub fn search<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Search, size, color)
}

pub fn palette<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Palette, size, color)
}

pub fn zap<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Zap, size, color)
}

pub fn chevron_down<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::ChevronDown, size, color)
}

pub fn arrow_down<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::ArrowDown, size, color)
}

pub fn arrow_up<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::ArrowUp, size, color)
}

pub fn square_plus<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::SquarePlus, size, color)
}

pub fn columns<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Columns2, size, color)
}

pub fn copy<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Copy, size, color)
}

pub fn check<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Check, size, color)
}

pub fn info<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Info, size, color)
}

pub fn alert_triangle<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::TriangleAlert, size, color)
}

#[allow(dead_code)]
pub fn maximize_box<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Square, size, color)
}

pub fn file_text<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::FileText, size, color)
}

pub fn folder<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Folder, size, color)
}

pub fn folder_plus<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::FolderPlus, size, color)
}

pub fn file_plus<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::FilePlus, size, color)
}

pub fn refresh_cw<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::RefreshCw, size, color)
}

pub fn arrow_left<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::ArrowLeft, size, color)
}

pub fn arrow_right<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::ArrowRight, size, color)
}

pub fn rotate_ccw<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::RotateCcw, size, color)
}

pub fn clipboard<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Clipboard, size, color)
}

pub fn save<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Save, size, color)
}

pub fn external_link<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::ExternalLink, size, color)
}

pub fn square<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Square, size, color)
}

pub fn rotate_cw<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::RotateCw, size, color)
}

pub fn sliders<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::SlidersHorizontal, size, color)
}

pub fn layers<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Layers, size, color)
}

pub fn sparkles<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Sparkles, size, color)
}

pub fn keyboard<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Keyboard, size, color)
}

pub fn maximize_2<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Maximize2, size, color)
}

pub fn plug<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Plug, size, color)
}

pub fn unlink<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Unlink, size, color)
}

pub fn scissors<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Scissors, size, color)
}

pub fn folder_tree<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::FolderTree, size, color)
}

pub fn users<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Users, size, color)
}

pub fn radio<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Radio, size, color)
}

pub fn circle_dot<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::CircleDot, size, color)
}

pub fn command<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Command, size, color)
}

pub fn cloud_download<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::CloudDownload, size, color)
}

pub fn cable<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Cable, size, color)
}

pub fn globe<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Globe, size, color)
}

pub fn lock_open<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::LockOpen, size, color)
}

pub fn arrow_right_left<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::ArrowRightLeft, size, color)
}

pub fn highlighter<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Highlighter, size, color)
}

pub fn tag<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::Tag, size, color)
}

pub fn file_down<'a>(size: f32, color: Color) -> text::Text<'a> {
    icon(Icon::FileDown, size, color)
}
