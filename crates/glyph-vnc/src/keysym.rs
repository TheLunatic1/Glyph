use iced::keyboard::{key::Named, Key};

pub const KEYSYM_BACKSPACE: u32 = 0xFF08;
pub const KEYSYM_TAB: u32 = 0xFF09;
pub const KEYSYM_RETURN: u32 = 0xFF0D;
pub const KEYSYM_ESCAPE: u32 = 0xFF1B;
pub const KEYSYM_HOME: u32 = 0xFF50;
pub const KEYSYM_LEFT: u32 = 0xFF51;
pub const KEYSYM_UP: u32 = 0xFF52;
pub const KEYSYM_RIGHT: u32 = 0xFF53;
pub const KEYSYM_DOWN: u32 = 0xFF54;
pub const KEYSYM_PAGE_UP: u32 = 0xFF55;
pub const KEYSYM_PAGE_DOWN: u32 = 0xFF56;
pub const KEYSYM_END: u32 = 0xFF57;
pub const KEYSYM_INSERT: u32 = 0xFF63;
pub const KEYSYM_F1: u32 = 0xFFBE;
pub const KEYSYM_SHIFT_L: u32 = 0xFFE1;
pub const KEYSYM_CTRL_L: u32 = 0xFFE3;
pub const KEYSYM_ALT_L: u32 = 0xFFE9;
pub const KEYSYM_SUPER_L: u32 = 0xFFEB;
pub const KEYSYM_DELETE: u32 = 0xFFFF;

/// Maps an Iced keyboard key + optional text fragment to an X11 KeySym.
pub fn iced_key_to_keysym(key: &Key, text: Option<&str>) -> Option<u32> {
    match key {
        Key::Named(named) => match named {
            Named::Backspace => Some(KEYSYM_BACKSPACE),
            Named::Tab => Some(KEYSYM_TAB),
            Named::Enter => Some(KEYSYM_RETURN),
            Named::Escape => Some(KEYSYM_ESCAPE),
            Named::Space => Some(0x0020),
            Named::Home => Some(KEYSYM_HOME),
            Named::ArrowLeft => Some(KEYSYM_LEFT),
            Named::ArrowUp => Some(KEYSYM_UP),
            Named::ArrowRight => Some(KEYSYM_RIGHT),
            Named::ArrowDown => Some(KEYSYM_DOWN),
            Named::PageUp => Some(KEYSYM_PAGE_UP),
            Named::PageDown => Some(KEYSYM_PAGE_DOWN),
            Named::End => Some(KEYSYM_END),
            Named::Insert => Some(KEYSYM_INSERT),
            Named::Delete => Some(KEYSYM_DELETE),
            Named::F1 => Some(KEYSYM_F1),
            Named::F2 => Some(KEYSYM_F1 + 1),
            Named::F3 => Some(KEYSYM_F1 + 2),
            Named::F4 => Some(KEYSYM_F1 + 3),
            Named::F5 => Some(KEYSYM_F1 + 4),
            Named::F6 => Some(KEYSYM_F1 + 5),
            Named::F7 => Some(KEYSYM_F1 + 6),
            Named::F8 => Some(KEYSYM_F1 + 7),
            Named::F9 => Some(KEYSYM_F1 + 8),
            Named::F10 => Some(KEYSYM_F1 + 9),
            Named::F11 => Some(KEYSYM_F1 + 10),
            Named::F12 => Some(KEYSYM_F1 + 11),
            Named::Shift => Some(KEYSYM_SHIFT_L),
            Named::Control => Some(KEYSYM_CTRL_L),
            Named::Alt => Some(KEYSYM_ALT_L),
            Named::Super => Some(KEYSYM_SUPER_L),
            _ => None,
        },
        Key::Character(s) => {
            let ch = text
                .and_then(|t| t.chars().next())
                .or_else(|| s.chars().next())?;
            char_to_keysym(ch)
        }
        _ => None,
    }
}

pub fn char_to_keysym(ch: char) -> Option<u32> {
    let cp = ch as u32;
    if (0x20..=0x7E).contains(&cp) || (0xA0..=0xFF).contains(&cp) {
        Some(cp)
    } else if cp > 0xFF {
        Some(0x0100_0000 | cp)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_named_and_unicode_keys() {
        assert_eq!(
            iced_key_to_keysym(&Key::Named(Named::Enter), None),
            Some(KEYSYM_RETURN)
        );
        assert_eq!(
            iced_key_to_keysym(&Key::Named(Named::F5), None),
            Some(KEYSYM_F1 + 4)
        );
        assert_eq!(
            iced_key_to_keysym(&Key::Character("a".into()), Some("a")),
            Some(b'a' as u32)
        );
        assert_eq!(char_to_keysym('€'), Some(0x0100_20AC));
    }
}
