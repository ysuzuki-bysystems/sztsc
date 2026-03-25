use std::ptr;
use std::slice;

use super::FreerdpError;
use super::Result;
use super::lib;

// TODO bindgen
const PIXEL_FORMAT_BGRA32: u32 = 537168008;

pub struct Invalid<'a> {
    raw: &'a mut lib::GDI_RGN,
}

impl<'a> Invalid<'a> {
    pub fn null(&self) -> bool {
        self.raw.null != 0
    }

    pub fn set_null(&mut self, val: bool) {
        self.raw.null = val as i32;
    }

    pub fn x(&self) -> i32 {
        self.raw.x
    }

    pub fn y(&self) -> i32 {
        self.raw.y
    }

    pub fn w(&self) -> i32 {
        self.raw.w
    }

    pub fn h(&self) -> i32 {
        self.raw.h
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PixelFormat {
    Bgr32,
}

#[repr(transparent)]
pub struct Gdi<'a> {
    raw: &'a mut lib::rdp_gdi,
}

impl<'a> Gdi<'a> {
    pub(super) fn from_raw(raw: *mut lib::rdp_gdi) -> Option<Gdi<'a>> {
        let Some(mut raw) = ptr::NonNull::new(raw) else {
            return None;
        };
        let raw = unsafe { raw.as_mut() };

        Some(Self { raw })
    }

    pub fn invalid(&self) -> Option<Invalid<'a>> {
        let raw = &self.raw;

        let mut primay = ptr::NonNull::new(raw.primary).unwrap();
        let primay = unsafe { primay.as_mut() };
        let mut hdc = ptr::NonNull::new(primay.hdc).unwrap();
        let hdc = unsafe { hdc.as_mut() };
        let Some(mut hwnd) = ptr::NonNull::new(hdc.hwnd) else {
            return None;
        };
        let hwnd = unsafe { hwnd.as_mut() };

        let mut invalid = ptr::NonNull::new(hwnd.invalid).unwrap();
        let invalid = unsafe { invalid.as_mut() };
        Some(Invalid { raw: invalid })
    }

    pub fn primary_buffer(&self) -> &[u8] {
        let raw = &self.raw;
        unsafe {
            slice::from_raw_parts(
                raw.primary_buffer,
                raw.stride as usize * raw.height as usize,
            )
        }
    }

    pub fn width(&self) -> i32 {
        self.raw.width
    }

    pub fn height(&self) -> i32 {
        self.raw.height
    }

    pub fn stride(&self) -> u32 {
        self.raw.stride
    }

    pub fn suppress_output(&self) -> bool {
        self.raw.suppressOutput != 0
    }
}

pub struct OwnedGdi {
    instance: ptr::NonNull<lib::freerdp>,
}

impl Drop for OwnedGdi {
    fn drop(&mut self) {
        unsafe { lib::gdi_free(self.instance.as_ptr()) };
    }
}

impl OwnedGdi {
    pub(super) fn init(instance: ptr::NonNull<lib::freerdp>, format: PixelFormat) -> Result<Self> {
        let format = match format {
            PixelFormat::Bgr32 => PIXEL_FORMAT_BGRA32,
        };

        let result = unsafe { lib::gdi_init(instance.as_ptr(), format) };
        if result == 0 {
            return Err(FreerdpError::InitGdi);
        }

        Ok(Self { instance })
    }
}
