use super::types::KeyPress;
use crate::gui::input::KeyCode;

/// Render a keypress in a user-friendly format (e.g. "Ctrl + G").
pub(crate) fn format_keypress(press: &KeyPress) -> String {
    let mut parts: Vec<&'static str> = Vec::new();
    if press.command {
        parts.push(command_label());
    }
    if press.shift {
        parts.push("Shift");
    }
    if press.alt {
        parts.push("Alt");
    }
    parts.push(key_label(press.key));
    parts.join(" + ")
}

fn command_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}

fn key_label(key: KeyCode) -> &'static str {
    match key {
        KeyCode::Num0 => "0",
        KeyCode::Num1 => "1",
        KeyCode::Num2 => "2",
        KeyCode::Num3 => "3",
        KeyCode::Num4 => "4",
        KeyCode::Num5 => "5",
        KeyCode::Num6 => "6",
        KeyCode::Num7 => "7",
        KeyCode::Num8 => "8",
        KeyCode::Num9 => "9",
        KeyCode::X => "X",
        KeyCode::N => "N",
        KeyCode::D => "D",
        KeyCode::C => "C",
        KeyCode::R => "R",
        KeyCode::T => "T",
        KeyCode::U => "U",
        KeyCode::Y => "Y",
        KeyCode::Z => "Z",
        KeyCode::M => "M",
        KeyCode::B => "B",
        KeyCode::Slash => "/",
        KeyCode::Backslash => "\\",
        KeyCode::Quote => "'",
        KeyCode::G => "G",
        KeyCode::S => "S",
        KeyCode::W => "W",
        KeyCode::L => "L",
        KeyCode::P => "P",
        KeyCode::F => "F",
        KeyCode::F1 => "F1",
        KeyCode::OpenBracket => "[",
        KeyCode::CloseBracket => "]",
        KeyCode::ArrowLeft => "Left",
        KeyCode::ArrowRight => "Right",
        KeyCode::ArrowUp => "Up",
        KeyCode::ArrowDown => "Down",
        _ => "Key",
    }
}
