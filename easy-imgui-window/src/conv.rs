/*!
* Helper module to convert between common types.
*/

use easy_imgui as imgui;
use easy_imgui_sys::*;
use winit::{
    event::MouseButton,
    keyboard::{KeyCode, PhysicalKey},
    window::CursorIcon,
};

/// Convert a `winit` mouse button to the `imgui` one.
pub fn to_imgui_button(btncode: MouseButton) -> Option<imgui::MouseButton> {
    let btn = match btncode {
        MouseButton::Left => imgui::MouseButton::Left,
        MouseButton::Right => imgui::MouseButton::Right,
        MouseButton::Middle => imgui::MouseButton::Middle,
        MouseButton::Other(x) if x < ImGuiMouseButton_::ImGuiMouseButton_COUNT.0 as u16 => {
            imgui::MouseButton::Other(x)
        }
        _ => return None,
    };
    Some(btn)
}

/// Convert an `winit` key to the `imgui` one.
pub fn to_imgui_key(phys_key: PhysicalKey) -> Option<imgui::Key> {
    let key = match phys_key {
        PhysicalKey::Code(code) => match code {
            KeyCode::Tab => imgui::Key::Tab,
            KeyCode::ArrowLeft => imgui::Key::LeftArrow,
            KeyCode::ArrowRight => imgui::Key::RightArrow,
            KeyCode::ArrowUp => imgui::Key::UpArrow,
            KeyCode::ArrowDown => imgui::Key::DownArrow,
            KeyCode::PageUp => imgui::Key::PageUp,
            KeyCode::PageDown => imgui::Key::PageDown,
            KeyCode::Home => imgui::Key::Home,
            KeyCode::End => imgui::Key::End,
            KeyCode::Insert => imgui::Key::Insert,
            KeyCode::Delete => imgui::Key::Delete,
            KeyCode::Backspace => imgui::Key::Backspace,
            KeyCode::Space => imgui::Key::Space,
            KeyCode::Enter => imgui::Key::Enter,
            KeyCode::Escape => imgui::Key::Escape,
            KeyCode::ControlLeft => imgui::Key::LeftCtrl,
            KeyCode::ShiftLeft => imgui::Key::LeftShift,
            KeyCode::AltLeft => imgui::Key::LeftAlt,
            KeyCode::SuperLeft => imgui::Key::LeftSuper,
            KeyCode::ControlRight => imgui::Key::RightCtrl,
            KeyCode::ShiftRight => imgui::Key::RightShift,
            KeyCode::AltRight => imgui::Key::RightAlt,
            KeyCode::SuperRight => imgui::Key::RightSuper,
            KeyCode::Digit0 => imgui::Key::Num0,
            KeyCode::Digit1 => imgui::Key::Num1,
            KeyCode::Digit2 => imgui::Key::Num2,
            KeyCode::Digit3 => imgui::Key::Num3,
            KeyCode::Digit4 => imgui::Key::Num4,
            KeyCode::Digit5 => imgui::Key::Num5,
            KeyCode::Digit6 => imgui::Key::Num6,
            KeyCode::Digit7 => imgui::Key::Num7,
            KeyCode::Digit8 => imgui::Key::Num8,
            KeyCode::Digit9 => imgui::Key::Num9,
            KeyCode::KeyA => imgui::Key::A,
            KeyCode::KeyB => imgui::Key::B,
            KeyCode::KeyC => imgui::Key::C,
            KeyCode::KeyD => imgui::Key::D,
            KeyCode::KeyE => imgui::Key::E,
            KeyCode::KeyF => imgui::Key::F,
            KeyCode::KeyG => imgui::Key::G,
            KeyCode::KeyH => imgui::Key::H,
            KeyCode::KeyI => imgui::Key::I,
            KeyCode::KeyJ => imgui::Key::J,
            KeyCode::KeyK => imgui::Key::K,
            KeyCode::KeyL => imgui::Key::L,
            KeyCode::KeyM => imgui::Key::M,
            KeyCode::KeyN => imgui::Key::N,
            KeyCode::KeyO => imgui::Key::O,
            KeyCode::KeyP => imgui::Key::P,
            KeyCode::KeyQ => imgui::Key::Q,
            KeyCode::KeyR => imgui::Key::R,
            KeyCode::KeyS => imgui::Key::S,
            KeyCode::KeyT => imgui::Key::T,
            KeyCode::KeyU => imgui::Key::U,
            KeyCode::KeyV => imgui::Key::V,
            KeyCode::KeyW => imgui::Key::W,
            KeyCode::KeyX => imgui::Key::X,
            KeyCode::KeyY => imgui::Key::Y,
            KeyCode::KeyZ => imgui::Key::Z,
            KeyCode::F1 => imgui::Key::F1,
            KeyCode::F2 => imgui::Key::F2,
            KeyCode::F3 => imgui::Key::F3,
            KeyCode::F4 => imgui::Key::F4,
            KeyCode::F5 => imgui::Key::F5,
            KeyCode::F6 => imgui::Key::F6,
            KeyCode::F7 => imgui::Key::F7,
            KeyCode::F8 => imgui::Key::F8,
            KeyCode::F9 => imgui::Key::F9,
            KeyCode::F10 => imgui::Key::F10,
            KeyCode::F11 => imgui::Key::F11,
            KeyCode::F12 => imgui::Key::F12,
            KeyCode::Quote => imgui::Key::Apostrophe,
            KeyCode::Comma => imgui::Key::Comma,
            KeyCode::Minus => imgui::Key::Minus,
            KeyCode::Period => imgui::Key::Period,
            KeyCode::Slash => imgui::Key::Slash,
            KeyCode::Semicolon => imgui::Key::Semicolon,
            KeyCode::Equal => imgui::Key::Equal,
            KeyCode::BracketLeft => imgui::Key::LeftBracket,
            KeyCode::Backslash => imgui::Key::Backslash,
            KeyCode::BracketRight => imgui::Key::RightBracket,
            KeyCode::Backquote => imgui::Key::GraveAccent,
            KeyCode::CapsLock => imgui::Key::CapsLock,
            KeyCode::ScrollLock => imgui::Key::ScrollLock,
            KeyCode::NumLock => imgui::Key::NumLock,
            KeyCode::PrintScreen => imgui::Key::PrintScreen,
            KeyCode::Pause => imgui::Key::Pause,
            KeyCode::Numpad0 => imgui::Key::Keypad0,
            KeyCode::Numpad1 => imgui::Key::Keypad1,
            KeyCode::Numpad2 => imgui::Key::Keypad2,
            KeyCode::Numpad3 => imgui::Key::Keypad3,
            KeyCode::Numpad4 => imgui::Key::Keypad4,
            KeyCode::Numpad5 => imgui::Key::Keypad5,
            KeyCode::Numpad6 => imgui::Key::Keypad6,
            KeyCode::Numpad7 => imgui::Key::Keypad7,
            KeyCode::Numpad8 => imgui::Key::Keypad8,
            KeyCode::Numpad9 => imgui::Key::Keypad9,
            KeyCode::NumpadDecimal => imgui::Key::KeypadDecimal,
            KeyCode::NumpadDivide => imgui::Key::KeypadDivide,
            KeyCode::NumpadMultiply => imgui::Key::KeypadMultiply,
            KeyCode::NumpadSubtract => imgui::Key::KeypadSubtract,
            KeyCode::NumpadAdd => imgui::Key::KeypadAdd,
            KeyCode::NumpadEnter => imgui::Key::KeypadEnter,
            KeyCode::NumpadEqual => imgui::Key::KeypadEqual,
            KeyCode::NumpadBackspace => imgui::Key::Backspace,
            _ => return None,
        },
        PhysicalKey::Unidentified(_) => return None,
    };
    Some(key)
}

/// Convert a mouse cursor from the `imgui` enum to the `winit` one.
pub fn from_imgui_cursor(cursor: imgui::MouseCursor) -> Option<CursorIcon> {
    use CursorIcon::*;
    let c = match cursor {
        imgui::MouseCursor::Arrow => Default,
        imgui::MouseCursor::TextInput => Text,
        imgui::MouseCursor::ResizeAll => Move,
        imgui::MouseCursor::ResizeNS => NsResize,
        imgui::MouseCursor::ResizeEW => EwResize,
        imgui::MouseCursor::ResizeNESW => NeswResize,
        imgui::MouseCursor::ResizeNWSE => NwseResize,
        imgui::MouseCursor::Hand => Pointer,
        imgui::MouseCursor::NotAllowed => NotAllowed,
        imgui::MouseCursor::None => return None,
    };
    Some(c)
}
