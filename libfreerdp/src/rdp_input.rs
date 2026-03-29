use std::ffi::c_int;
use std::marker::PhantomData;
use std::ptr;

use super::FreerdpError;
use super::Result;
use super::lib;

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
}
