use core::slice;
use std::ffi::CStr;
use std::ffi::CString;
use std::ffi::NulError;
use std::mem;
use std::ptr;
use std::str::FromStr;
use std::vec;

use super::lib;
use bitflags::bitflags;
use thiserror::Error;

unsafe extern "C" fn monitor_ready(
    cx: *mut lib::CliprdrClientContext,
    _monitor_ready: *const lib::CLIPRDR_MONITOR_READY,
) -> lib::UINT {
    let cx = ptr::NonNull::new(cx).unwrap();
    let mut cx = CliprdrClientContext::from_raw(cx);

    let Some((cx, cb)) = cx.callbacks_mut() else {
        return 0;
    };

    match cb.monitor_ready(cx, MonitorReady) {
        Ok(()) => 0,
        Err(err) => {
            eprint!("{err}");
            return 1;
        }
    }
}

unsafe extern "C" fn server_capabilities(
    cx: *mut lib::CliprdrClientContext,
    capabilities: *const lib::CLIPRDR_CAPABILITIES,
) -> lib::UINT {
    let cx = ptr::NonNull::new(cx).unwrap();
    let mut cx = CliprdrClientContext::from_raw(cx);

    let Some((cx, cb)) = cx.callbacks_mut() else {
        return 0;
    };

    let capabilities = Capabilities::from_raw(capabilities);

    match cb.server_capabilities(cx, capabilities) {
        Ok(()) => 0,
        Err(err) => {
            eprint!("{err}");
            return 1;
        }
    }
}

unsafe extern "C" fn server_format_list(
    cx: *mut lib::CliprdrClientContext,
    format_list: *const lib::CLIPRDR_FORMAT_LIST,
) -> lib::UINT {
    let cx = ptr::NonNull::new(cx).unwrap();
    let mut cx = CliprdrClientContext::from_raw(cx);

    let Some((cx, cb)) = cx.callbacks_mut() else {
        return 0;
    };

    let format_list = FormatList::from_raw(format_list);

    match cb.server_format_list(cx, format_list) {
        Ok(()) => 0,
        Err(err) => {
            eprint!("{err}");
            return 1;
        }
    }
}

unsafe extern "C" fn server_format_list_response(
    cx: *mut lib::CliprdrClientContext,
    format_list_response: *const lib::CLIPRDR_FORMAT_LIST_RESPONSE,
) -> lib::UINT {
    let cx = ptr::NonNull::new(cx).unwrap();
    let mut cx = CliprdrClientContext::from_raw(cx);

    let Some((cx, cb)) = cx.callbacks_mut() else {
        return 0;
    };

    let format_list = FormatListResponse::from_raw(format_list_response);

    match cb.server_format_list_response(cx, format_list) {
        Ok(()) => 0,
        Err(err) => {
            eprint!("{err}");
            return 1;
        }
    }
}

unsafe extern "C" fn server_format_data_request(
    cx: *mut lib::CliprdrClientContext,
    format_data_request: *const lib::CLIPRDR_FORMAT_DATA_REQUEST,
) -> lib::UINT {
    let cx = ptr::NonNull::new(cx).unwrap();
    let mut cx = CliprdrClientContext::from_raw(cx);

    let Some((cx, cb)) = cx.callbacks_mut() else {
        return 0;
    };

    let format_data_request = FormatDataRequest::from_raw(format_data_request);

    match cb.server_format_data_request(cx, format_data_request) {
        Ok(()) => 0,
        Err(err) => {
            eprint!("{err}");
            return 1;
        }
    }
}

unsafe extern "C" fn server_format_data_response(
    cx: *mut lib::CliprdrClientContext,
    format_data_response: *const lib::CLIPRDR_FORMAT_DATA_RESPONSE,
) -> lib::UINT {
    let cx = ptr::NonNull::new(cx).unwrap();
    let mut cx = CliprdrClientContext::from_raw(cx);

    let Some((cx, cb)) = cx.callbacks_mut() else {
        return 0;
    };

    let format_data_response = FormatDataResponse::from_raw(format_data_response);

    match cb.server_format_data_response(cx, format_data_response) {
        Ok(()) => 0,
        Err(err) => {
            eprint!("{err}");
            return 1;
        }
    }
}

#[derive(Debug)]
pub struct CliprdrClientContext {
    raw: ptr::NonNull<lib::CliprdrClientContext>,
}

#[derive(Debug, Error)]
pub enum CliprdrCallbackError {
    #[error("{0}")]
    Any(String),
}

pub type CliprdrCallbackResult<T> = std::result::Result<T, CliprdrCallbackError>;

pub struct MonitorReady;

bitflags! {
    #[derive(Debug)]
    pub struct GeneralCapabilityFlags: u32 {
        const CB_USE_LONG_FORMAT_NAMES = lib::CB_USE_LONG_FORMAT_NAMES;
        const CB_STREAM_FILECLIP_ENABLED = lib::CB_STREAM_FILECLIP_ENABLED;
        const CB_FILECLIP_NO_FILE_PATHS = lib::CB_FILECLIP_NO_FILE_PATHS;
        const CB_CAN_LOCK_CLIPDATA = lib::CB_CAN_LOCK_CLIPDATA;
        const CB_HUGE_FILE_SUPPORT_ENABLED = lib::CB_HUGE_FILE_SUPPORT_ENABLED;
    }
}

#[derive(Debug)]
pub struct GeneralCapability {
    flags: GeneralCapabilityFlags,
}

impl GeneralCapability {
    pub fn new(flags: GeneralCapabilityFlags) -> Self {
        Self { flags }
    }
}

#[derive(Debug)]
pub enum Capability {
    General(GeneralCapability),
    Unknown(Vec<u8>),
}

impl From<GeneralCapability> for Capability {
    fn from(value: GeneralCapability) -> Self {
        Self::General(value)
    }
}

impl Capability {
    fn len(&self) -> usize {
        match self {
            Self::General(_) => 12,
            Self::Unknown(v) => 4 + v.len(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Capabilities {
    capabilities: Vec<Capability>,
}

impl Capabilities {
    pub fn add<C: Into<Capability>>(&mut self, capability: C) {
        self.capabilities.push(capability.into());
    }

    fn from_raw(raw: *const lib::CLIPRDR_CAPABILITIES) -> Self {
        let raw = unsafe { raw.as_ref() }.unwrap();
        let mut capabilities = vec![];
        let mut off = 0;
        for _ in 0..raw.cCapabilitiesSets {
            let cap = unsafe { raw.capabilitySets.offset(off).as_mut() }.unwrap();
            match cap.capabilitySetType as u32 {
                lib::CB_CAPSTYPE_GENERAL => {
                    if cap.capabilitySetLength != lib::CB_CAPSTYPE_GENERAL_LEN as u16 {
                        panic!()
                    }
                    let cap = unsafe {
                        mem::transmute::<_, &mut lib::CLIPRDR_GENERAL_CAPABILITY_SET>(cap)
                    };
                    capabilities.push(Capability::General(GeneralCapability {
                        flags: GeneralCapabilityFlags::from_bits_retain(cap.generalFlags),
                    }));
                    off += cap.capabilitySetLength as isize;
                }
                _ => {
                    let data = vec![]; // TODO
                    capabilities.push(Capability::Unknown(data));
                    off += cap.capabilitySetLength as isize;
                }
            }
        }

        Self { capabilities }
    }
}

#[repr(u32)]
#[derive(Debug)]
pub enum FormatId {
    CfRaw = lib::CF_RAW,
    CfText = lib::CF_TEXT,
    CfBitmap = lib::CF_BITMAP,
    CfMetafilepict = lib::CF_METAFILEPICT,
    CfSylk = lib::CF_SYLK,
    CfDif = lib::CF_DIF,
    CfTiff = lib::CF_TIFF,
    CfOemtext = lib::CF_OEMTEXT,
    CfDib = lib::CF_DIB,
    CfPalette = lib::CF_PALETTE,
    CfPendata = lib::CF_PENDATA,
    CfRiff = lib::CF_RIFF,
    CfWave = lib::CF_WAVE,
    CfUnicodetext = lib::CF_UNICODETEXT,
    CfEnhmetafile = lib::CF_ENHMETAFILE,
    CfHdrop = lib::CF_HDROP,
    CfLocale = lib::CF_LOCALE,
    CfDibv5 = lib::CF_DIBV5,
    CfMax = lib::CF_MAX,
    CfOwnerdisplay = lib::CF_OWNERDISPLAY,
    CfDsptext = lib::CF_DSPTEXT,
    CfDspbitmap = lib::CF_DSPBITMAP,
    CfDspmetafilepict = lib::CF_DSPMETAFILEPICT,
    CfDspenhmetafile = lib::CF_DSPENHMETAFILE,
    CfPrivatefirst = lib::CF_PRIVATEFIRST,
    CfPrivatelast = lib::CF_PRIVATELAST,
    CfGdiobjfirst = lib::CF_GDIOBJFIRST,
    CfGdiobjlast = lib::CF_GDIOBJLAST,
}

impl FormatId {
    fn try_from(val: u32) -> Option<Self> {
        Some(match val {
            lib::CF_RAW => Self::CfRaw,
            lib::CF_TEXT => Self::CfText,
            lib::CF_BITMAP => Self::CfBitmap,
            lib::CF_METAFILEPICT => Self::CfMetafilepict,
            lib::CF_SYLK => Self::CfSylk,
            lib::CF_DIF => Self::CfDif,
            lib::CF_TIFF => Self::CfTiff,
            lib::CF_OEMTEXT => Self::CfOemtext,
            lib::CF_DIB => Self::CfDib,
            lib::CF_PALETTE => Self::CfPalette,
            lib::CF_PENDATA => Self::CfPendata,
            lib::CF_RIFF => Self::CfRiff,
            lib::CF_WAVE => Self::CfWave,
            lib::CF_UNICODETEXT => Self::CfUnicodetext,
            lib::CF_ENHMETAFILE => Self::CfEnhmetafile,
            lib::CF_HDROP => Self::CfHdrop,
            lib::CF_LOCALE => Self::CfLocale,
            lib::CF_DIBV5 => Self::CfDibv5,
            lib::CF_MAX => Self::CfMax,
            lib::CF_OWNERDISPLAY => Self::CfOwnerdisplay,
            lib::CF_DSPTEXT => Self::CfDsptext,
            lib::CF_DSPBITMAP => Self::CfDspbitmap,
            lib::CF_DSPMETAFILEPICT => Self::CfDspmetafilepict,
            lib::CF_DSPENHMETAFILE => Self::CfDspenhmetafile,
            lib::CF_PRIVATEFIRST => Self::CfPrivatefirst,
            lib::CF_PRIVATELAST => Self::CfPrivatelast,
            lib::CF_GDIOBJFIRST => Self::CfGdiobjfirst,
            lib::CF_GDIOBJLAST => Self::CfGdiobjlast,
            _ => return None,
        })
    }
}

#[derive(Debug)]
pub enum Format {
    Id(FormatId),
    Name(CString),
}

impl Format {
    pub fn new_with_name(val: &str) -> CliprdrResult<Self> {
        let v = CString::from_str(val)?;
        Ok(Self::Name(v))
    }
}

impl From<FormatId> for Format {
    fn from(value: FormatId) -> Self {
        Self::Id(value)
    }
}

#[derive(Debug, Default)]
pub struct FormatList {
    formats: Vec<Format>,
}

impl FormatList {
    pub fn add<F: Into<Format>>(&mut self, format: F) {
        self.formats.push(format.into());
    }

    fn from_raw(raw: *const lib::CLIPRDR_FORMAT_LIST) -> Self {
        let raw = unsafe { raw.as_ref() }.unwrap();
        let formats = unsafe { slice::from_raw_parts(raw.formats, raw.numFormats as usize) };
        let formats = formats
            .into_iter()
            .map(|v| {
                if v.formatName.is_null() {
                    let id = FormatId::try_from(v.formatId as u32).unwrap();
                    Format::Id(id)
                } else {
                    let name = unsafe { CStr::from_ptr(v.formatName) };
                    Format::Name(name.into())
                }
            })
            .collect();
        Self { formats }
    }
}

#[repr(u32)]
#[derive(Debug)]
pub enum FormatListResponse {
    Ok = lib::CB_RESPONSE_OK,
    Fail = lib::CB_RESPONSE_FAIL,
}

impl FormatListResponse {
    fn from_raw(raw: *const lib::CLIPRDR_FORMAT_LIST_RESPONSE) -> Self {
        let raw = unsafe { raw.as_ref() }.unwrap();
        match raw.common.msgFlags as u32 {
            lib::CB_RESPONSE_OK => Self::Ok,
            lib::CB_RESPONSE_FAIL => Self::Fail,
            _ => panic!(),
        }
    }
}

#[derive(Debug)]
pub struct FormatDataRequest(FormatId);

impl FormatDataRequest {
    pub fn new(id: FormatId) -> Self {
        Self(id)
    }

    fn from_raw(raw: *const lib::CLIPRDR_FORMAT_DATA_REQUEST) -> Self {
        let raw = unsafe { raw.as_ref() }.unwrap();
        let id = FormatId::try_from(raw.requestedFormatId).unwrap();
        Self(id)
    }
}

#[derive(Debug)]
pub enum FormatDataResponse<'a> {
    Ok(&'a [u8]),
    Fail,
}

impl FormatDataResponse<'_> {
    fn from_raw(raw: *const lib::CLIPRDR_FORMAT_DATA_RESPONSE) -> Self {
        let raw = unsafe { raw.as_ref() }.unwrap();

        match raw.common.msgFlags as u32 {
            lib::CB_RESPONSE_OK => {
                let len = raw.common.dataLen as usize;
                let data = unsafe { slice::from_raw_parts(raw.requestedFormatData, len) };
                Self::Ok(data)
            }
            lib::CB_RESPONSE_FAIL => Self::Fail,
            _ => panic!(),
        }
    }
}

pub trait CliprdrCallbacks {
    fn monitor_ready(
        &mut self,
        context: &mut CliprdrClientContext,
        monitor_ready: MonitorReady,
    ) -> CliprdrCallbackResult<()>;
    fn server_capabilities(
        &mut self,
        context: &mut CliprdrClientContext,
        capabilities: Capabilities,
    ) -> CliprdrCallbackResult<()>;
    fn server_format_list(
        &mut self,
        context: &mut CliprdrClientContext,
        format_list: FormatList,
    ) -> CliprdrCallbackResult<()>;
    fn server_format_list_response(
        &mut self,
        context: &mut CliprdrClientContext,
        format_list_response: FormatListResponse,
    ) -> CliprdrCallbackResult<()>;
    fn server_format_data_request(
        &mut self,
        context: &mut CliprdrClientContext,
        format_data_request: FormatDataRequest,
    ) -> CliprdrCallbackResult<()>;
    fn server_format_data_response(
        &mut self,
        context: &mut CliprdrClientContext,
        format_data_response: FormatDataResponse,
    ) -> CliprdrCallbackResult<()>;
}

struct CallbackHolder(Box<dyn CliprdrCallbacks>);

#[derive(Debug, Error)]
pub enum CliprdrError {
    #[error("{0}")]
    Nul(#[from] NulError),
}

pub type CliprdrResult<T> = std::result::Result<T, CliprdrError>;

impl CliprdrClientContext {
    pub(super) fn from_raw(raw: ptr::NonNull<lib::CliprdrClientContext>) -> Self {
        Self { raw }
    }

    fn callbacks_mut<'a>(&'a mut self) -> Option<(&'a mut Self, &'a mut dyn CliprdrCallbacks)> {
        let raw = unsafe { self.raw.as_mut() };
        if raw.custom.is_null() {
            return None;
        }

        let callbacks = unsafe { Box::from_raw(raw.custom.cast::<CallbackHolder>()) };
        let callbacks = mem::ManuallyDrop::new(callbacks).as_mut().0.as_mut() as *mut _;
        Some((self, unsafe { &mut *callbacks }))
    }

    pub(super) fn drop_context_custom(mut raw: ptr::NonNull<lib::CliprdrClientContext>) {
        let raw = unsafe { raw.as_mut() };
        if raw.custom.is_null() {
            return;
        }

        drop(unsafe { Box::from_raw(raw.custom) });
    }

    pub fn set_callbacks<C: CliprdrCallbacks + 'static>(&mut self, callbacks: C) {
        let raw = unsafe { self.raw.as_mut() };
        if !raw.custom.is_null() {
            panic!("Already set CliprdrCallbacks")
        }

        raw.MonitorReady = Some(monitor_ready);
        raw.ServerCapabilities = Some(server_capabilities);
        raw.ServerFormatList = Some(server_format_list);
        raw.ServerFormatListResponse = Some(server_format_list_response);
        raw.ServerFormatDataRequest = Some(server_format_data_request);
        raw.ServerFormatDataResponse = Some(server_format_data_response);

        raw.custom = Box::into_raw(Box::new(CallbackHolder(Box::new(callbacks)))).cast();
    }

    pub fn client_capabilities(&mut self, capabilities: Capabilities) -> CliprdrResult<()> {
        let capset_len = capabilities.capabilities.iter().map(Capability::len).sum();
        let mut capset = vec![0u8; capset_len];

        let mut p = 0;
        for cap in &capabilities.capabilities {
            match cap {
                Capability::General(v) => {
                    let raw = lib::CLIPRDR_GENERAL_CAPABILITY_SET {
                        capabilitySetType: lib::CB_CAPSTYPE_GENERAL as u16,
                        capabilitySetLength: lib::CB_CAPSTYPE_GENERAL_LEN as u16,
                        version: lib::CB_CAPS_VERSION_2,
                        generalFlags: v.flags.bits(),
                    };
                    let len = size_of::<lib::CLIPRDR_GENERAL_CAPABILITY_SET>();
                    unsafe { ptr::write(capset[p..p + len].as_mut_ptr().cast(), raw) };
                    p += len;
                }

                Capability::Unknown(_) => panic!("Not supported"),
            }
        }

        let value = lib::CLIPRDR_CAPABILITIES {
            common: lib::CLIPRDR_HEADER {
                msgType: 0,
                dataLen: 0,
                msgFlags: 0,
            },
            cCapabilitiesSets: capabilities.capabilities.len() as u32,
            capabilitySets: capset.as_mut_ptr().cast(),
        };

        let cx = unsafe { self.raw.as_mut() };
        if (unsafe { cx.ClientCapabilities.unwrap()(cx, &value) }) != 0 {
            todo!()
        }

        Ok(())
    }

    pub fn client_format_list(&mut self, format_list: FormatList) -> CliprdrResult<()> {
        let formats = format_list.formats;
        let mut formats = formats
            .into_iter()
            .map(|v| match v {
                Format::Id(id) => lib::CLIPRDR_FORMAT {
                    formatId: id as u32,
                    formatName: ptr::null_mut(),
                },

                Format::Name(name) => lib::CLIPRDR_FORMAT {
                    formatId: 0,
                    formatName: name.as_ptr() as *mut _,
                },
            })
            .collect::<Vec<_>>();
        let format_list = lib::CLIPRDR_FORMAT_LIST {
            common: lib::CLIPRDR_HEADER {
                msgType: lib::CliprdrMsgType_CB_FORMAT_LIST as lib::UINT16,
                dataLen: 0,
                msgFlags: 0,
            },
            numFormats: formats.len() as lib::UINT,
            formats: formats.as_mut_ptr(),
        };

        let cx = unsafe { self.raw.as_mut() };
        if (unsafe { cx.ClientFormatList.unwrap()(cx, &format_list) }) != 0 {
            todo!()
        }

        Ok(())
    }

    pub fn client_format_list_response(
        &mut self,
        format_list_response: FormatListResponse,
    ) -> CliprdrResult<()> {
        let response = lib::CLIPRDR_FORMAT_LIST_RESPONSE {
            common: lib::CLIPRDR_HEADER {
                msgType: lib::CliprdrMsgType_CB_FORMAT_LIST_RESPONSE as u16,
                dataLen: 0,
                msgFlags: format_list_response as u16,
            },
        };

        let cx = unsafe { self.raw.as_mut() };
        if (unsafe { cx.ClientFormatListResponse.unwrap()(cx, &response) }) != 0 {
            todo!()
        }

        Ok(())
    }

    pub fn client_format_data_request(
        &mut self,
        format_data_request: FormatDataRequest,
    ) -> CliprdrResult<()> {
        let request = lib::CLIPRDR_FORMAT_DATA_REQUEST {
            common: lib::CLIPRDR_HEADER {
                msgType: lib::CliprdrMsgType_CB_FORMAT_DATA_REQUEST as u16,
                dataLen: 0,
                msgFlags: 0,
            },
            requestedFormatId: format_data_request.0 as u32,
        };

        let cx = unsafe { self.raw.as_mut() };
        if (unsafe { cx.ClientFormatDataRequest.unwrap()(cx, &request) }) != 0 {
            todo!()
        }

        Ok(())
    }

    pub fn client_format_data_response(
        &mut self,
        format_data_response: FormatDataResponse,
    ) -> CliprdrResult<()> {
        let response = match format_data_response {
            FormatDataResponse::Ok(data) => lib::CLIPRDR_FORMAT_DATA_RESPONSE {
                common: lib::CLIPRDR_HEADER {
                    msgType: lib::CliprdrMsgType_CB_FORMAT_DATA_RESPONSE as u16,
                    dataLen: data.len() as lib::UINT32,
                    msgFlags: lib::CB_RESPONSE_OK as u16,
                },
                requestedFormatData: data.as_ptr(),
            },

            FormatDataResponse::Fail => lib::CLIPRDR_FORMAT_DATA_RESPONSE {
                common: lib::CLIPRDR_HEADER {
                    msgType: lib::CliprdrMsgType_CB_FORMAT_DATA_RESPONSE as u16,
                    dataLen: 0,
                    msgFlags: lib::CB_RESPONSE_FAIL as u16,
                },
                requestedFormatData: ptr::null_mut(),
            },
        };

        let cx = unsafe { self.raw.as_mut() };
        if (unsafe { cx.ClientFormatDataResponse.unwrap()(cx, &response) }) != 0 {
            todo!()
        }

        Ok(())
    }
}
