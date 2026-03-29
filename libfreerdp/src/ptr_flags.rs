use super::lib;

bitflags::bitflags! {
    pub struct PtrFlags: lib::UINT16 {
        const HWHEEL = lib::PTR_FLAGS_HWHEEL as u16;
        const WHEEL = lib::PTR_FLAGS_WHEEL as u16;
        const WHEEL_NEGATIVE = lib::PTR_FLAGS_WHEEL_NEGATIVE as u16;
        const MOVE = lib::PTR_FLAGS_MOVE as u16;
        const DOWN = lib::PTR_FLAGS_DOWN as u16;
        const BUTTON1 = lib::PTR_FLAGS_BUTTON1 as u16;
        const BUTTON2 = lib::PTR_FLAGS_BUTTON2 as u16;
        const BUTTON3 = lib::PTR_FLAGS_BUTTON3 as u16;
    }
}
