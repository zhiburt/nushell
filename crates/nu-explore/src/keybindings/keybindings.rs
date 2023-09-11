use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// parse a string as a keyboard key definition.
///
/// examples:
///     "g" -> 'G'
///     "ALT_g" -> 'ALT_G'
///     "ALT_SHIFT_CTRL_g" -> 'ALT_SHIFT_CTRL_G'
///     "CTRL_ALT_SHIFT_g" -> 'ALT_SHIFT_CTRL_G'
pub fn parse_key(input: &str) -> Option<KeyEvent> {
    let mut key = KeyEvent::new(KeyCode::Home, KeyModifiers::empty());

    let mut tokens = input.split('_').rev();

    let mut code = Cow::Borrowed(tokens.next()?);
    let is_uppercase_letter = code.len() == 1 && code.chars().next().unwrap().is_uppercase();
    if is_uppercase_letter {
        code = Cow::Owned(code.to_lowercase());
    }

    key.code = parse_code(&code)?;

    for mods in tokens {
        parse_modifier(&mut key.modifiers, mods)?;
    }

    if is_uppercase_letter {
        key.modifiers |= KeyModifiers::SHIFT;
    }

    Some(key)
}

fn parse_modifier(mods: &mut KeyModifiers, token: &str) -> Option<()> {
    match token.to_ascii_lowercase().as_ref() {
        "ctrl" => {
            mods.insert(KeyModifiers::CONTROL);
        }
        "alt" => {
            mods.insert(KeyModifiers::ALT);
        }
        "shift" => {
            mods.insert(KeyModifiers::SHIFT);
        }
        _ => {
            return None;
        }
    }

    Some(())
}

fn parse_code(code: &str) -> Option<KeyCode> {
    use KeyCode::*;

    let code = match code {
        "ESC" => Esc,
        "ENTER" => Enter,
        "LEFT" => Left,
        "RIGHT" => Right,
        "UP" => Up,
        "DOWN" => Down,
        "HOME" => Home,
        "END" => End,
        "PAGEUP" => PageUp,
        "PAGEDOWN" => PageDown,
        "BACKTAB" => BackTab,
        "BACKSPACE" => Backspace,
        "DEL" => Delete,
        "INS" => Insert,
        "F1" => F(1),
        "F2" => F(2),
        "F3" => F(3),
        "F4" => F(4),
        "F5" => F(5),
        "F6" => F(6),
        "F7" => F(7),
        "F8" => F(8),
        "F9" => F(9),
        "F10" => F(10),
        "F11" => F(11),
        "F12" => F(12),
        "SPACE" => Char(' '),
        "TAB" => Tab,
        str if str.len() == 1 => Char(str.chars().next().unwrap()),
        _ => {
            return None;
        }
    };

    Some(code)
}
