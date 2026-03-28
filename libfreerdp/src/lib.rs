use std::os::fd::RawFd;
use std::ptr;
use std::result::Result as StdResult;

use libfreerdp_sys as lib;
use thiserror::Error;

pub use callbacks::CallbackError;
pub use callbacks::CallbackResult;
pub use callbacks::Callbacks;
pub use disp_client_context::DispClientContext;
pub use freerdp::Freerdp;
pub use gdi::Gdi;
pub use gdi::Invalid;
pub use gdi::OwnedGdi;
pub use gdi::PixelFormat;
pub use lib::HANDLE;
pub use rdp_context::OwnedRdpContext;
pub use rdp_context::RdpContext;
pub use rdp_input::PtrFlags;
pub use rdp_input::RdpInput;
pub use settings::Settings;
pub use dvc::Dvc;

mod callbacks;
mod disp_client_context;
mod freerdp;
mod gdi;
mod rdp_context;
mod rdp_input;
mod settings;
mod trampolines;
mod dvc;

// FIXME
#[derive(Debug, Error)]
pub enum FreerdpError {
    #[error("freerdp_new")]
    FreerdpNew,

    #[error("freerdp_settings_new")]
    FreerdpSettingsNew,

    #[error("new_client_context")]
    NewClientContext,

    #[error("freerdp_context_new_ex")]
    FreerdpContextNewEx,

    #[error("freerdp_connect")]
    FreerdpConnect,

    #[error("drdynvc: null")]
    NoDrdynvc,

    #[error("init_gdi")]
    InitGdi,

    #[error("failed to setting: {0}")]
    SettingsSet(String),

    #[error("alread created")]
    AlreadyCreated,

    #[error("alread connected")]
    AlreadyConnected,

    #[error("WaitForMultipleObjects")]
    WaitForMultipleObjects,

    #[error("freerdp_input_send_keyboard_event")]
    FreerdpInputSendKeyboardEvent,

    #[error("freerdp_register_addin_provider")]
    FreerdpRegisterAddinProvider,
}

pub type Result<T> = StdResult<T, FreerdpError>;

pub fn fd_to_handle(fd: RawFd) -> HANDLE {
    unsafe { lib::CreateFileDescriptorEventW(ptr::null_mut(), 0, 0, fd, 1) }
}

pub fn poll<T: AsRef<[HANDLE]>>(events: T) -> Result<()> {
    let events = events.as_ref();
    let r =
        unsafe { lib::WaitForMultipleObjects(events.len() as u32, events.as_ptr(), 0, 0xFFFFFFFF) };
    if r == 0xFFFFFFFF {
        return Err(FreerdpError::WaitForMultipleObjects);
    }
    Ok(())
}

pub fn new_client_context<C: Callbacks + 'static>(callbacks: C) -> Result<OwnedRdpContext> {
    OwnedRdpContext::new_client_context(callbacks)
}
