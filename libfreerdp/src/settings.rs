use std::ffi::CString;
use std::ffi::c_int;
use std::ptr;

use super::FreerdpError;
use super::Result;
use super::lib;

#[repr(transparent)]
pub struct Settings {
    pub(super) raw: ptr::NonNull<lib::rdp_settings>,
}

impl Settings {
    pub fn new() -> Result<Self> {
        let raw = unsafe { lib::freerdp_settings_new(0) };
        let Some(raw) = ptr::NonNull::new(raw) else {
            return Err(FreerdpError::FreerdpSettingsNew);
        };

        Ok(Self { raw })
    }

    pub fn set_server_host_name(&mut self, val: &str) {
        self.set_string(
            lib::FreeRDP_Settings_Keys_String_FreeRDP_ServerHostname,
            val,
        )
    }

    pub fn set_server_port(&mut self, val: u16) {
        self.set_u32(
            lib::FreeRDP_Settings_Keys_UInt32_FreeRDP_ServerPort,
            val as u32,
        )
    }

    pub fn set_username(&mut self, val: &str) {
        self.set_string(lib::FreeRDP_Settings_Keys_String_FreeRDP_Username, val)
    }

    pub fn set_password(&mut self, val: &str) {
        self.set_string(lib::FreeRDP_Settings_Keys_String_FreeRDP_Password, val);
    }

    pub fn set_aad_security(&mut self, val: bool) {
        self.set_bool(lib::FreeRDP_Settings_Keys_Bool_FreeRDP_AadSecurity, val);
    }

    pub fn set_keyboard_layout(&mut self, val: u32) {
        self.set_u32(
            lib::FreeRDP_Settings_Keys_UInt32_FreeRDP_KeyboardLayout,
            val,
        );
    }

    pub fn set_keyboard_type(&mut self, val: u32) {
        self.set_u32(lib::FreeRDP_Settings_Keys_UInt32_FreeRDP_KeyboardType, val);
    }

    pub fn set_keyboard_subtype(&mut self, val: u32) {
        self.set_u32(
            lib::FreeRDP_Settings_Keys_UInt32_FreeRDP_KeyboardSubType,
            val,
        );
    }

    pub fn set_keyboard_function_key(&mut self, val: u32) {
        self.set_u32(
            lib::FreeRDP_Settings_Keys_UInt32_FreeRDP_KeyboardFunctionKey,
            val,
        );
    }

    pub fn set_dynamic_resolution_update(&mut self, val: bool) {
        self.set_bool(
            lib::FreeRDP_Settings_Keys_Bool_FreeRDP_DynamicResolutionUpdate,
            val,
        );
    }

    pub fn get_desktop_width(&self) -> u32 {
        self.get_u32(lib::FreeRDP_Settings_Keys_UInt32_FreeRDP_DesktopWidth)
    }

    pub fn get_desktop_height(&self) -> u32 {
        self.get_u32(lib::FreeRDP_Settings_Keys_UInt32_FreeRDP_DesktopHeight)
    }

    fn set_string(&mut self, id: lib::FreeRDP_Settings_Keys_String, v: &str) {
        let val = CString::new(v).unwrap();
        let result =
            unsafe { lib::freerdp_settings_set_string(self.raw.as_ptr(), id, val.as_ptr()) };
        if result != 1 {
            panic!("{id}");
        }
    }

    fn set_u32(&mut self, id: lib::FreeRDP_Settings_Keys_UInt32, v: u32) {
        let result = unsafe { lib::freerdp_settings_set_uint32(self.raw.as_ptr(), id, v) };
        if result != 1 {
            panic!("{id}");
        }
    }

    fn set_bool(&mut self, id: lib::FreeRDP_Settings_Keys_Bool, v: bool) {
        let result = unsafe { lib::freerdp_settings_set_bool(self.raw.as_ptr(), id, v as c_int) };
        if result != 1 {
            panic!("{id}");
        }
    }

    fn get_u32(&self, id: lib::FreeRDP_Settings_Keys_UInt32) -> u32 {
        return unsafe { lib::freerdp_settings_get_uint32(self.raw.as_ptr(), id) };
    }
}
