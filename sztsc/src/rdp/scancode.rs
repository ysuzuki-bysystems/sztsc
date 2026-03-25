use winit::keyboard::KeyCode;

pub(super) fn to_rdp_scancode(key: KeyCode) -> Option<u32> {
    use KeyCode::*;

    let (code, extended) = match key {
        // letters
        KeyA => (0x1E, false),
        KeyB => (0x30, false),
        KeyC => (0x2E, false),
        KeyD => (0x20, false),
        KeyE => (0x12, false),
        KeyF => (0x21, false),
        KeyG => (0x22, false),
        KeyH => (0x23, false),
        KeyI => (0x17, false),
        KeyJ => (0x24, false),
        KeyK => (0x25, false),
        KeyL => (0x26, false),
        KeyM => (0x32, false),
        KeyN => (0x31, false),
        KeyO => (0x18, false),
        KeyP => (0x19, false),
        KeyQ => (0x10, false),
        KeyR => (0x13, false),
        KeyS => (0x1F, false),
        KeyT => (0x14, false),
        KeyU => (0x16, false),
        KeyV => (0x2F, false),
        KeyW => (0x11, false),
        KeyX => (0x2D, false),
        KeyY => (0x15, false),
        KeyZ => (0x2C, false),

        // digits
        Digit1 => (0x02, false),
        Digit2 => (0x03, false),
        Digit3 => (0x04, false),
        Digit4 => (0x05, false),
        Digit5 => (0x06, false),
        Digit6 => (0x07, false),
        Digit7 => (0x08, false),
        Digit8 => (0x09, false),
        Digit9 => (0x0A, false),
        Digit0 => (0x0B, false),

        // punctuation
        Minus => (0x0C, false),        // RDP_SCANCODE_OEM_MINUS
        Equal => (0x0D, false),        // RDP_SCANCODE_OEM_PLUS
        BracketLeft => (0x1A, false),  // RDP_SCANCODE_OEM_4
        BracketRight => (0x1B, false), // RDP_SCANCODE_OEM_6
        Backslash => (0x2B, false),    // RDP_SCANCODE_OEM_5
        Semicolon => (0x27, false),    // RDP_SCANCODE_OEM_1
        Quote => (0x28, false),        // RDP_SCANCODE_OEM_7
        Backquote => (0x29, false),    // RDP_SCANCODE_OEM_3
        Comma => (0x33, false),        // RDP_SCANCODE_OEM_COMMA
        Period => (0x34, false),       // RDP_SCANCODE_OEM_PERIOD
        Slash => (0x35, false),        // RDP_SCANCODE_OEM_2

        // controls
        Enter => (0x1C, false),
        Tab => (0x0F, false),
        Space => (0x39, false),
        Backspace => (0x0E, false),
        Escape => (0x01, false),
        CapsLock => (0x3A, false),

        // modifiers
        ShiftLeft => (0x2A, false),
        ShiftRight => (0x36, false),
        ControlLeft => (0x1D, false),
        ControlRight => (0x1D, true),
        AltLeft => (0x38, false),
        AltRight => (0x38, true),
        SuperLeft => (0x5B, true),
        SuperRight => (0x5C, true),
        ContextMenu => (0x5D, true),

        // arrows
        ArrowUp => (0x48, true),
        ArrowDown => (0x50, true),
        ArrowLeft => (0x4B, true),
        ArrowRight => (0x4D, true),

        // navigation
        Insert => (0x52, true),
        Delete => (0x53, true),
        Home => (0x47, true),
        End => (0x4F, true),
        PageUp => (0x49, true),
        PageDown => (0x51, true),

        // function keys
        F1 => (0x3B, false),
        F2 => (0x3C, false),
        F3 => (0x3D, false),
        F4 => (0x3E, false),
        F5 => (0x3F, false),
        F6 => (0x40, false),
        F7 => (0x41, false),
        F8 => (0x42, false),
        F9 => (0x43, false),
        F10 => (0x44, false),
        F11 => (0x57, false),
        F12 => (0x58, false),

        // keypad
        Numpad0 => (0x52, false),
        Numpad1 => (0x4F, false),
        Numpad2 => (0x50, false),
        Numpad3 => (0x51, false),
        Numpad4 => (0x4B, false),
        Numpad5 => (0x4C, false),
        Numpad6 => (0x4D, false),
        Numpad7 => (0x47, false),
        Numpad8 => (0x48, false),
        Numpad9 => (0x49, false),
        NumpadAdd => (0x4E, false),
        NumpadSubtract => (0x4A, false),
        NumpadMultiply => (0x37, false),
        NumpadDivide => (0x35, true),
        NumpadDecimal => (0x53, false),
        NumpadEnter => (0x1C, true),

        // JIS
        IntlYen => (0x2B, false),
        IntlRo => (0x73, false),
        KanaMode => (0x70, false),

        _ => return None,
    };

    const KBDEXT: u32 = 0x0100;

    Some(if extended { code | KBDEXT } else { code })
}
