use std::cell::RefCell;
use std::cell::RefMut;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr;
use std::rc::Rc;

use super::Callbacks;
use super::Gdi;
use super::HANDLE;
use super::OwnedGdi;
use super::RdpInput;
use super::Settings;
use super::lib;

#[repr(C)]
pub(super) struct RawRdpContext {
    pub(super) common: lib::rdp_context,
    pub(super) callbacks: MaybeUninit<Rc<RefCell<Box<dyn Callbacks>>>>,
    pub(super) initialized_gdi: Option<OwnedGdi>,
}

impl RawRdpContext {
    pub(super) fn callbacks_mut<'a>(&'a self) -> RefMut<'a, Box<dyn Callbacks + 'static>> {
        let callbacks = unsafe { self.callbacks.assume_init_ref() };
        callbacks.borrow_mut()
    }
}

#[repr(transparent)]
pub struct RdpContext {
    pub(super) raw: ptr::NonNull<RawRdpContext>,
}

impl RdpContext {
    pub(super) fn new(raw: ptr::NonNull<RawRdpContext>) -> Self {
        Self { raw }
    }

    pub fn settings(self) -> Settings {
        let raw = unsafe { self.raw.as_ref() }.common.settings;
        let raw = ptr::NonNull::new(raw).unwrap();
        Settings { raw }
    }

    pub fn gdi<'a>(&'a self) -> Option<Gdi<'a>> {
        let raw = unsafe { self.raw.as_ref() };
        Gdi::from_raw(raw.common.gdi)
    }

    pub fn input<'a>(&'a self) -> RdpInput<'a> {
        let raw = unsafe { self.raw.as_ref() };
        RdpInput::from_raw(raw.common.input).unwrap()
    }
}

#[repr(transparent)]
pub struct OwnedRdpContext {
    pub(super) inner: RdpContext,
}

impl Drop for OwnedRdpContext {
    fn drop(&mut self) {
        let raw = unsafe { self.inner.raw.as_mut() };
        unsafe {
            lib::freerdp_context_free(raw.common.instance);
        }
    }
}

impl AsRef<RdpContext> for OwnedRdpContext {
    fn as_ref(&self) -> &RdpContext {
        &self.inner
    }
}

impl AsMut<RdpContext> for OwnedRdpContext {
    fn as_mut(&mut self) -> &mut RdpContext {
        &mut self.inner
    }
}

impl OwnedRdpContext {
    pub fn shall_disconnect(&mut self) -> bool {
        let raw = self.inner.raw;
        let r = unsafe { lib::freerdp_shall_disconnect_context(raw.as_ptr() as *const _) };
        r != 0
    }

    pub fn get_event_handles<T: AsMut<[HANDLE]>>(&mut self, mut events: T) -> usize {
        let events = events.as_mut();
        let raw = self.inner.raw;
        let r = unsafe {
            lib::freerdp_get_event_handles(
                raw.as_ptr() as *mut _,
                events.as_mut_ptr() as *mut *mut c_void,
                events.len() as u32,
            )
        };

        r as usize
    }

    pub fn check_event_handles(&mut self) -> bool {
        let raw = self.inner.raw;
        let r = unsafe { lib::freerdp_check_event_handles(raw.as_ptr() as *mut _) };
        r != 0
    }
}
