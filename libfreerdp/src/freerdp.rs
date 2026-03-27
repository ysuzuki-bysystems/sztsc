use std::cell::RefCell;
use std::ptr;
use std::ptr::NonNull;
use std::rc::Rc;

use super::Callbacks;
use super::FreerdpError;
use super::OwnedGdi;
use super::OwnedRdpContext;
use super::PixelFormat;
use super::RdpContext;
use super::Result;
use super::Settings;
use super::lib;
use super::rdp_context::RawRdpContext;
use super::trampolines;

pub struct Connection<'a> {
    instance: &'a OwnedFreerdp,
}

impl<'a> Drop for Connection<'a> {
    fn drop(&mut self) {
        let raw = self.instance.inner.raw.as_ptr();
        unsafe { lib::freerdp_disconnect(raw) };
    }
}

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

pub struct OwnedFreerdp {
    inner: Freerdp,
    callbacks: Rc<RefCell<Box<dyn Callbacks>>>,
}

impl Drop for OwnedFreerdp {
    fn drop(&mut self) {
        let raw = self.inner.raw.as_ptr();
        unsafe { lib::freerdp_free(raw) };
    }
}

impl OwnedFreerdp {
    pub fn new<C: Callbacks + 'static>(callbacks: C) -> Result<Self> {
        let raw = unsafe { lib::freerdp_new() };
        let Some(mut raw) = ptr::NonNull::new(raw) else {
            return Err(FreerdpError::FreerdpNew);
        };

        let p = unsafe { raw.as_mut() };
        p.ContextSize = size_of::<RawRdpContext>();
        trampolines::setup_instance(p)?;

        let inner = Freerdp { raw };
        let result = OwnedFreerdp {
            inner,
            callbacks: Rc::new(RefCell::new(Box::new(callbacks))),
        };
        Ok(result)
    }

    pub fn new_context_ex(&mut self, settings: &Settings) -> Result<OwnedRdpContext> {
        let raw = unsafe { self.inner.raw.as_mut() };
        let result = unsafe { lib::freerdp_context_new_ex(raw, settings.raw.as_ptr()) };
        if result == 0 {
            return Err(FreerdpError::FreerdpContextNewEx);
        }

        let Some(mut cx) = ptr::NonNull::new(raw.context as *mut RawRdpContext) else {
            return Err(FreerdpError::FreerdpContextNewEx);
        };
        unsafe { cx.as_mut() }
            .callbacks
            .write(self.callbacks.clone());

        let inner = RdpContext::new(cx);
        let context = OwnedRdpContext { inner };

        Ok(context)
    }

    pub fn connect<'a>(&'a mut self) -> Result<Connection<'a>> {
        let raw = self.inner.raw.as_ptr();
        let result = unsafe { lib::freerdp_connect(raw) };
        if result == 0 {
            return Err(FreerdpError::FreerdpConnect);
        }

        Ok(Connection { instance: self })
    }
}
