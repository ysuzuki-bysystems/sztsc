use std::ffi::CStr;
use std::ffi::c_int;
use std::ffi::c_void;
use std::mem;
use std::mem::MaybeUninit;
use std::ptr;

use crate::FreerdpError;

use super::Callbacks;
use super::DispClientContext;
use super::Dvc;
use super::Gdi;
use super::HANDLE;
use super::OwnedGdi;
use super::RdpInput;
use super::Result;
use super::Settings;
use super::lib;
use super::trampolines;

#[repr(C)]
pub(super) struct RawRdpContext {
    pub(super) common: lib::rdpClientContext,
    pub(super) callbacks: MaybeUninit<Box<dyn Callbacks>>,
    pub(super) initialized_gdi: Option<OwnedGdi>,
}

impl RawRdpContext {
    pub(super) fn callbacks_mut(&mut self) -> &mut Box<dyn Callbacks + 'static> {
        unsafe { self.callbacks.assume_init_mut() }
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

    pub fn settings(&self) -> Settings {
        let raw = unsafe { self.raw.as_ref() }.common.context.settings;
        let raw = ptr::NonNull::new(raw).unwrap();
        Settings { raw }
    }

    pub fn gdi<'a>(&'a self) -> Option<Gdi<'a>> {
        let raw = unsafe { self.raw.as_ref() };
        Gdi::from_raw(raw.common.context.gdi)
    }

    pub fn input<'a>(&'a self) -> RdpInput<'a> {
        let raw = unsafe { self.raw.as_ref() };
        RdpInput::from_raw(raw.common.context.input).unwrap()
    }

    pub fn disp_client_context(&mut self) -> Option<DispClientContext> {
        let raw = self.raw.as_ptr().cast();
        let raw = unsafe { lib::freerdp_client_get_instance(raw).cast::<lib::DispClientContext>() };
        let Some(raw) = ptr::NonNull::new(raw) else {
            return None;
        };

        Some(DispClientContext::from_raw(raw))
    }

    pub fn send_button_event(&mut self, relative: bool, flags: u16, x: i32, y: i32) -> Result<()> {
        let raw = unsafe { self.raw.as_mut() };

        unsafe {
            lib::freerdp_client_send_button_event(
                &mut raw.common,
                relative as lib::BOOL,
                flags,
                x,
                y,
            )
        };
        Ok(())
    }
}

pub struct OwnedRdpContext {
    pub(super) inner: RdpContext,
    connected: bool,
}

impl Drop for OwnedRdpContext {
    fn drop(&mut self) {
        if self.connected {
            let raw = unsafe { self.inner.raw.as_mut() };
            unsafe { lib::freerdp_disconnect(raw.common.context.instance) };
        }

        unsafe {
            lib::freerdp_client_context_free(self.inner.raw.as_ptr().cast());
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

unsafe extern "C" fn global_init() -> lib::BOOL {
    1
}
unsafe extern "C" fn global_uninit() {}
unsafe extern "C" fn client_new(
    instance: *mut lib::freerdp,
    _context: *mut lib::rdpContext,
) -> lib::BOOL {
    let Some(mut instance) = ptr::NonNull::new(instance) else {
        eprintln!("instance: null");
        return 0;
    };
    let instance = unsafe { instance.as_mut() };
    trampolines::setup_instance(instance);

    1
}
unsafe extern "C" fn client_free(_instance: *mut lib::freerdp, context: *mut lib::rdpContext) {
    let Some(mut context) = ptr::NonNull::new(context.cast::<RawRdpContext>()) else {
        return;
    };
    let context = unsafe { context.as_mut() };
    // drop callbacks
    let _ = mem::replace(&mut context.callbacks, MaybeUninit::zeroed());
}

unsafe extern "C" fn on_channel_connected(
    cx: *mut c_void,
    e: *mut lib::ChannelConnectedEventArgs,
) -> c_int {
    let cx = cx.cast::<RawRdpContext>();
    let cx = unsafe { cx.as_mut() }.unwrap();

    let e = unsafe { e.as_mut() }.unwrap();
    let name = unsafe { CStr::from_ptr(e.name) };

    match name.to_bytes_with_nul() {
        name if lib::DISP_DVC_CHANNEL_NAME == name => {
            let Some(raw) = ptr::NonNull::new(e.pInterface.cast::<lib::DispClientContext>()) else {
                eprintln!("p_interface: null");
                return 1;
            };
            let dvc = Dvc::Disp(DispClientContext::from_raw(raw));
            if let Err(err) = cx.callbacks_mut().on_channel_connected(dvc) {
                eprintln!("{err}");
                return 1;
            };
        }

        _ => {
            unsafe {
                lib::freerdp_client_OnChannelConnectedEventHandler(cx as *mut _ as *mut c_void, e)
            };
        }
    }

    0
}

unsafe extern "C" fn on_channel_disconnected(
    cx: *mut c_void,
    e: *mut lib::ChannelDisconnectedEventArgs,
) -> c_int {
    let cx = cx.cast::<RawRdpContext>();
    let cx = unsafe { cx.as_mut() }.unwrap();

    let e = unsafe { e.as_mut() }.unwrap();
    let name = unsafe { CStr::from_ptr(e.name) };

    match name.to_bytes_with_nul() {
        name if lib::DISP_DVC_CHANNEL_NAME == name => {
            let Some(raw) = ptr::NonNull::new(e.pInterface.cast::<lib::DispClientContext>()) else {
                eprintln!("p_interface: null");
                return 1;
            };
            let dvc = Dvc::Disp(DispClientContext::from_raw(raw));
            if let Err(err) = cx.callbacks_mut().on_channel_disconnected(dvc) {
                eprintln!("{err}");
                return 1;
            };
        }

        _ => {
            unsafe {
                lib::freerdp_client_OnChannelDisconnectedEventHandler(
                    cx as *mut _ as *mut c_void,
                    e,
                )
            };
        }
    }

    0
}

impl OwnedRdpContext {
    pub(super) fn new_client_context<C: Callbacks + 'static>(
        callbacks: C,
        settings: Settings,
    ) -> Result<Self> {
        let settings = settings.raw.as_ptr();

        let mut entrypoint = lib::RDP_CLIENT_ENTRY_POINTS {
            Size: size_of::<lib::RDP_CLIENT_ENTRY_POINTS>() as lib::DWORD,
            Version: lib::RDP_CLIENT_INTERFACE_VERSION,
            ContextSize: size_of::<RawRdpContext>() as lib::DWORD,
            GlobalInit: Some(global_init),
            GlobalUninit: Some(global_uninit),
            ClientNew: Some(client_new),
            ClientFree: Some(client_free),
            ClientStart: None,
            ClientStop: None,
            settings,
        };
        let context = unsafe { lib::freerdp_client_context_new(&mut entrypoint) };
        let Some(mut context) = ptr::NonNull::new(context.cast::<RawRdpContext>()) else {
            return Err(FreerdpError::NewClientContext);
        };
        let r = unsafe { context.as_mut() };
        r.callbacks.write(Box::new(callbacks));
        r.initialized_gdi = None;

        Ok(OwnedRdpContext {
            inner: RdpContext::new(context),
            connected: false,
        })
    }

    pub fn connect(&mut self) -> Result<()> {
        if self.connected {
            return Err(FreerdpError::AlreadyConnected);
        }

        let raw = unsafe { self.inner.raw.as_mut() };
        let result = unsafe { lib::freerdp_connect(raw.common.context.instance) };
        if result == 0 {
            return Err(FreerdpError::FreerdpConnect);
        }

        self.connected = true;

        let pubsub = raw.common.context.pubSub;
        unsafe { super::pubsub::subscribe_channel_connected(pubsub, on_channel_connected) };
        unsafe { super::pubsub::subscribe_channel_disconnected(pubsub, on_channel_disconnected) };

        Ok(())
    }

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
