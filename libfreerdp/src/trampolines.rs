use core::slice;
use std::ffi::CStr;
use std::ffi::CString;
use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;
use std::os::raw::c_uint;
use std::ptr;
use std::ptr::NonNull;

use super::Freerdp;
use super::RdpContext;
use super::lib;
use super::rdp_context::RawRdpContext;
use super::Result;
use super::FreerdpError;

unsafe extern "C" fn begin_paint(context: *mut lib::rdp_context) -> lib::BOOL {
    let mut raw = NonNull::new(context as *mut RawRdpContext).unwrap();
    let context = unsafe { raw.as_mut() };
    let mut callbacks = context.callbacks_mut();

    if let Err(err) = callbacks.begin_paint(&mut RdpContext::new(raw)) {
        eprint!("{err}");
        return 0;
    }

    1
}

unsafe extern "C" fn end_paint(context: *mut lib::rdp_context) -> lib::BOOL {
    let mut raw = NonNull::new(context as *mut RawRdpContext).unwrap();
    let context = unsafe { raw.as_mut() };
    let mut callbacks = context.callbacks_mut();

    if let Err(err) = callbacks.end_paint(&mut RdpContext::new(raw)) {
        eprint!("{err}");
        return 0;
    }

    1
}

unsafe extern "C" fn on_channel_connected(_cx: *mut lib::DrdynvcClientContext, name: *const c_char, _channel: *mut c_void) -> c_uint {
    let name = unsafe { CStr::from_ptr(name) };
    println!("{name:?}");
    0
}

unsafe extern "C" fn load_channels(instance: *mut lib::rdp_freerdp) -> lib::BOOL {
    let raw = NonNull::new(instance).unwrap();
    let raw = unsafe { raw.as_ref() };

    let cx = NonNull::new(raw.context).unwrap();
    let cx = unsafe { cx.as_ref() };

    let channels = cx.channels;
    let settings = cx.settings;

    let r = unsafe {
        lib::freerdp_channels_load_plugin(
            channels,
            settings,
            lib::DRDYNVC_CHANNEL_NAME.as_ptr() as *const c_char,
            ptr::null_mut())
    };

    (r == 0) as lib::BOOL
}

unsafe extern "C" fn pre_connect(instance: *mut lib::rdp_freerdp) -> lib::BOOL {
    let mut raw = NonNull::new(instance).unwrap();
    let context = unsafe { raw.as_mut() }.context;
    let Some(mut context) = NonNull::new(context as *mut RawRdpContext) else {
        eprint!("context: null");
        return 0;
    };
    let context = unsafe { context.as_mut() };
    let mut callbacks = context.callbacks_mut();

    if let Err(err) = callbacks.pre_connect(&mut Freerdp::new(raw)) {
        eprint!("{err}");
        return 0;
    }

    1
}

unsafe extern "C" fn post_connect(instance: *mut lib::rdp_freerdp) -> lib::BOOL {
    let mut raw = NonNull::new(instance).unwrap();
    let context = unsafe { raw.as_mut() }.context;
    let Some(mut context) = ptr::NonNull::new(context as *mut RawRdpContext) else {
        eprint!("context: null");
        return 0;
    };
    let context = unsafe { context.as_mut() };
    let mut callbacks = context.callbacks_mut();

    let Some(mut update) = ptr::NonNull::new(context.common.update) else {
        eprint!("update: null");
        return 0;
    };
    let update = unsafe { update.as_mut() };
    update.BeginPaint = Some(begin_paint);
    update.EndPaint = Some(end_paint);

    let channels = context.common.channels;
    let dvc = unsafe {
        lib::freerdp_channels_get_static_channel_interface(channels, lib::DRDYNVC_CHANNEL_NAME.as_ptr() as *const c_char).cast::<lib::DrdynvcClientContext>()
    };
    let Some(mut dvc) = ptr::NonNull::new(dvc) else {
        eprint!("dvc: null");
        return 0;
    };
    let dvc = unsafe { dvc.as_mut() };
    dvc.OnChannelConnected = Some(on_channel_connected);

    if let Err(err) = callbacks.post_connect(&mut Freerdp::new(raw)) {
        eprint!("{err}");
        return 0;
    }

    1
}

unsafe extern "C" fn post_disconnect(instance: *mut lib::rdp_freerdp) {
    let mut raw = NonNull::new(instance).unwrap();
    let context = unsafe { raw.as_mut() }.context;
    let Some(mut context) = NonNull::new(context as *mut RawRdpContext) else {
        eprint!("context: null");
        return;
    };
    let context = unsafe { context.as_mut() };
    let mut callbacks = context.callbacks_mut();

    if let Err(err) = callbacks.post_disconnect(&mut Freerdp::new(raw)) {
        eprint!("{err}");
        return;
    }
}

unsafe extern "C" fn verify_x509_certificate(
    instance: *mut lib::freerdp,
    data: *const lib::BYTE,
    length: usize,
    hostname: *const c_char,
    port: lib::UINT16,
    flags: lib::DWORD,
) -> c_int {
    let mut raw = NonNull::new(instance).unwrap();
    let context = unsafe { raw.as_mut() }.context;
    let Some(mut context) = NonNull::new(context as *mut RawRdpContext) else {
        eprint!("context: null");
        return 0;
    };
    let context = unsafe { context.as_mut() };
    let mut callbacks = context.callbacks_mut();

    let data = unsafe { slice::from_raw_parts(data, length) };
    let hostname = unsafe { CStr::from_ptr(hostname as *mut _) };
    if let Err(err) = callbacks.verify_x509_certificate(
        &mut Freerdp::new(raw),
        data,
        &hostname.to_string_lossy(),
        port,
        flags,
    ) {
        // TODO
        eprintln!("{err}");
        return 0;
    };

    1
}

unsafe extern "C" {
    pub(super) fn get_access_token(
        instance: *mut lib::rdp_freerdp,
        token_type: lib::AccessTokenType,
        token: *mut *mut c_char,
        count: usize,
        ...
    ) -> i32;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_access_token_aad(
    instance: *mut lib::freerdp,
    token: *mut *mut c_char,
    scope: *mut c_char,
    req_cnf: *mut c_char,
) -> lib::BOOL {
    let mut raw = NonNull::new(instance).unwrap();
    let context = unsafe { raw.as_mut() }.context;
    let Some(mut context) = NonNull::new(context as *mut RawRdpContext) else {
        eprint!("context: null");
        return 0;
    };
    let context = unsafe { context.as_mut() };
    let mut callbacks = context.callbacks_mut();

    let scope = unsafe { CStr::from_ptr(scope) };
    let req_cnf = unsafe { CStr::from_ptr(req_cnf) };

    let t = match callbacks.get_access_token_aad(
        &mut Freerdp::new(raw),
        &scope.to_string_lossy(),
        &req_cnf.to_string_lossy(),
    ) {
        Ok(token) => token,
        Err(err) => {
            eprintln!("{err}");
            return 0;
        }
    };

    unsafe {
        *token = CString::new(t).unwrap().into_raw();
    };

    1
}

pub(super) fn setup_instance(instance: &mut lib::freerdp) -> Result<()> {
    // TODO move
    let r = unsafe {
        lib::freerdp_register_addin_provider(Some(lib::freerdp_channels_load_static_addin_entry), 0)
    };
    if r != 0 {
        return Err(FreerdpError::FreerdpRegisterAddinProvider);
    }

    instance.LoadChannels = Some(load_channels);
    instance.PreConnect = Some(pre_connect);
    instance.PostConnect = Some(post_connect);
    instance.PostDisconnect = Some(post_disconnect);
    instance.VerifyX509Certificate = Some(verify_x509_certificate);
    instance.GetAccessToken = Some(get_access_token);

    Ok(())
}
