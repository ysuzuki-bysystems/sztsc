use std::ffi::c_int;
use std::marker::PhantomData;
use std::ptr;

use super::FreerdpError;
use super::Result;
use super::lib;

bitflags::bitflags! {
    pub struct PtrFlags: u16 {
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

#[repr(transparent)]
pub struct RdpInput<'a> {
    raw: ptr::NonNull<lib::rdp_input>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> RdpInput<'a> {
    pub(super) fn from_raw(raw: *mut lib::rdp_input) -> Option<RdpInput<'a>> {
        let Some(raw) = ptr::NonNull::new(raw) else {
            return None;
        };
        Some(RdpInput {
            raw,
            _phantom: PhantomData,
        })
    }

    pub fn send_keyboard_event(&mut self, down: bool, repeat: bool, scan_code: u32) -> Result<()> {
        let result = unsafe {
            lib::freerdp_input_send_keyboard_event_ex(
                self.raw.as_ptr(),
                down as c_int,
                repeat as c_int,
                scan_code,
            )
        };
        if result == 0 {
            return Err(FreerdpError::FreerdpInputSendKeyboardEvent);
        }

        Ok(())
    }

    pub fn send_mouse_event(&mut self, flags: PtrFlags, x: u16, y: u16) -> Result<()> {
        let result =
            unsafe { lib::freerdp_input_send_mouse_event(self.raw.as_ptr(), flags.bits(), x, y) };
        if result == 0 {
            return Err(FreerdpError::FreerdpInputSendKeyboardEvent);
        }

        Ok(())
    }
}
