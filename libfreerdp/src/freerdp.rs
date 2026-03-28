use std::ptr;
use std::ptr::NonNull;

use super::FreerdpError;
use super::OwnedGdi;
use super::PixelFormat;
use super::RdpContext;
use super::Result;
use super::lib;
use super::rdp_context::RawRdpContext;

#[repr(transparent)]
pub struct Freerdp {
    raw: ptr::NonNull<lib::freerdp>,
}

impl Freerdp {
    pub(super) fn new(raw: ptr::NonNull<lib::freerdp>) -> Self {
        Self { raw }
    }

    pub fn context(&self) -> Option<RdpContext> {
        let p = unsafe { self.raw.as_ref() };
        let Some(context) = NonNull::new(p.context as *mut RawRdpContext) else {
            return None;
        };
        Some(RdpContext::new(context))
    }

    pub fn init_gdi(&mut self, format: PixelFormat) -> Result<()> {
        let Some(mut context) = self.context() else {
            return Err(FreerdpError::InitGdi);
        };

        let gdi = OwnedGdi::init(self.raw, format)?;
        unsafe { context.raw.as_mut() }.initialized_gdi = Some(gdi);

        Ok(())
    }

    pub fn free_gdi(&mut self) -> Result<()> {
        let Some(mut context) = self.context() else {
            return Err(FreerdpError::InitGdi);
        };

        unsafe { context.raw.as_mut() }.initialized_gdi = None;

        Ok(())
    }
}
