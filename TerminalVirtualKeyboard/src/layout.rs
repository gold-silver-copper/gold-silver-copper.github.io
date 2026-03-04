use ratatui::style::Color;
use std::sync::Arc;

use crate::virtual_key::VirtualKey;

#[derive(Debug)]
pub struct Layout {
    pub layer: Vec<Vec<Button>>,
}

#[derive(Debug)]
pub struct Attr {
    pub width: u16,
    pub height: u16,
    pub border_color: Option<Color>,
    pub highlight: Option<Color>,
}

impl Attr {
    pub fn default(name: &str) -> Self {
        let width = match name.to_lowercase().as_str() {
            "space" => 20,
            _ => 4,
        };
        Self {
            width,
            height: 3,
            border_color: None,
            highlight: None,
        }
    }

    pub fn with_width(width: u16) -> Self {
        Self {
            width,
            height: 3,
            border_color: None,
            highlight: None,
        }
    }
}

#[derive(Debug)]
pub struct Button {
    pub attr: Attr,
    pub binds: Vec<(Arc<str>, Option<VirtualKey>)>,
}

impl Button {
    /// Create a simple button with a single character binding.
    pub fn key(name: &str, vk: Option<VirtualKey>, width: u16) -> Self {
        Self {
            attr: Attr::with_width(width),
            binds: vec![(Arc::from(name), vk)],
        }
    }

    /// Create a button with a normal and shifted binding.
    pub fn key_shift(normal: &str, shifted: &str, vk: Option<VirtualKey>, width: u16) -> Self {
        Self {
            attr: Attr::with_width(width),
            binds: vec![
                (Arc::from(normal), vk),
                (Arc::from(shifted), Some(VirtualKey::ShiftLeft)),
            ],
        }
    }
}

/// Build a LISP-optimized keyboard layout.
///
/// The layout prioritizes parentheses, common Lisp symbols, and removes
/// unnecessary modifier keys (no alt, no ctrl). Single shift key for
/// accessing uppercase letters and additional symbols.
pub fn lisp_keyboard_layout() -> Layout {
    use VirtualKey::*;

    let k = Button::key;
    let ks = Button::key_shift;

    // Row 1: ( 1 2 3 4 5 6 7 8 9 )  0
    let row1 = vec![
        ks("(", "[", None, 3),
        ks("1", "@", Some(Num1), 3),
        ks("2", "#", Some(Num2), 3),
        ks("3", "$", Some(Num3), 3),
        ks("4", "%", Some(Num4), 3),
        ks("5", "^", Some(Num5), 3),
        ks("6", "&", Some(Num6), 3),
        ks("7", "<", Some(Num7), 3),
        ks("8", ">", Some(Num8), 3),
        ks("9", "~", Some(Num9), 3),
        ks(")", "]", None, 3),
        ks("0", "_", Some(Num0), 3),
    ];

    // Row 2: q w e r t y u i o p ⌫
    let row2 = vec![
        ks("q", "Q", Some(KeyQ), 3),
        ks("w", "W", Some(KeyW), 3),
        ks("e", "E", Some(KeyE), 3),
        ks("r", "R", Some(KeyR), 3),
        ks("t", "T", Some(KeyT), 3),
        ks("y", "Y", Some(KeyY), 3),
        ks("u", "U", Some(KeyU), 3),
        ks("i", "I", Some(KeyI), 3),
        ks("o", "O", Some(KeyO), 3),
        ks("p", "P", Some(KeyP), 3),
        k("⌫", Some(Backspace), 6),
    ];

    // Row 3: a s d f g h j k l ↵
    let row3 = vec![
        ks("a", "A", Some(KeyA), 3),
        ks("s", "S", Some(KeyS), 3),
        ks("d", "D", Some(KeyD), 3),
        ks("f", "F", Some(KeyF), 3),
        ks("g", "G", Some(KeyG), 3),
        ks("h", "H", Some(KeyH), 3),
        ks("j", "J", Some(KeyJ), 3),
        ks("k", "K", Some(KeyK), 3),
        ks("l", "L", Some(KeyL), 3),
        k("↵", Some(Return), 9),
    ];

    // Row 4: ⇧ z x c v b n m " .
    let row4 = vec![
        k("⇧", Some(ShiftLeft), 5),
        ks("z", "Z", Some(KeyZ), 3),
        ks("x", "X", Some(KeyX), 3),
        ks("c", "C", Some(KeyC), 3),
        ks("v", "V", Some(KeyV), 3),
        ks("b", "B", Some(KeyB), 3),
        ks("n", "N", Some(KeyN), 3),
        ks("m", "M", Some(KeyM), 3),
        ks("\"", "'", None, 3),
        ks(".", "=", None, 4),
    ];

    // Row 5: ! ? SPACE - + / *  :
    let row5 = vec![
        ks("!", "|", None, 3),
        ks("?", "\\", None, 3),
        k(" ", Some(Space), 12),
        ks("-", "_", None, 3),
        ks("+", "~", None, 3),
        ks("/", "\\", None, 3),
        ks("*", "^", None, 3),
        ks(":", ";", None, 4),
    ];

    Layout {
        layer: vec![row1, row2, row3, row4, row5],
    }
}
