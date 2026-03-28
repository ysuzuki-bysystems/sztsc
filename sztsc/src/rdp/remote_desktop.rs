use std::cell::RefCell;
use std::ops::ControlFlow;
use std::os::fd::AsRawFd;
use std::ptr;
use std::rc::Rc;

use thiserror::Error;
use winit::event::ElementState;
use winit::event::MouseButton;
use winit::event_loop::EventLoopClosed;
use winit::event_loop::EventLoopProxy;
use winit::keyboard::PhysicalKey;

use super::RdpEventSender;
use super::auth::build_auth_code_url;
use super::auth::get_code_by_webview;
use super::auth::get_token;
use super::event::RdpEventReciever;
use super::scancode::to_rdp_scancode;
use crate::event::UiEvent;
use crate::frame_buffer::FrameBuffer;
use crate::frame_buffer::SharedFrameBuffer;
use crate::rdp::RdpEvent;
use libfreerdp as lib;

#[derive(Debug, Error)]
pub enum RemoteDesktopError {
    #[error("{0}")]
    Freerdp(#[from] lib::FreerdpError),

    #[error("{0}")]
    EventLoopClosed(#[from] EventLoopClosed<UiEvent>),

    #[error("alread created")]
    AlreadyCreated,
}

type Result<T> = ::std::result::Result<T, RemoteDesktopError>;

#[derive(Debug)]
pub struct RemoteDesktop {
    pub(super) event_loop_proxy: EventLoopProxy<UiEvent>,
    auth_code_bin: String,
    fb: SharedFrameBuffer,
    event_tx: RdpEventSender,
    event_rx: Option<RdpEventReciever>,
    shared: Rc<RefCell<SharedContext>>,
}

impl lib::Callbacks for RemoteDesktop {
    fn get_access_token_aad(
        &mut self,
        _instance: &mut lib::Freerdp,
        scope: &str,
        req_cnf: &str,
    ) -> lib::CallbackResult<String> {
        let scope = percent_encoding::percent_decode_str(scope).collect::<Vec<_>>();
        let scope = String::from_utf8_lossy(&scope);

        let url = match build_auth_code_url(&scope) {
            Ok(url) => url,
            Err(err) => return Err(lib::CallbackError::Any(err.to_string())),
        };
        let code = match get_code_by_webview(&self.auth_code_bin, &url) {
            Ok(code) => code,
            Err(err) => return Err(lib::CallbackError::Any(err.to_string())),
        };
        let token = match get_token(&scope, req_cnf, &code) {
            Ok(token) => token,
            Err(err) => return Err(lib::CallbackError::Any(err.to_string())),
        };

        Ok(token)
    }

    fn verify_x509_certificate(
        &mut self,
        _instance: &mut lib::Freerdp,
        data: &[u8],
        hostname: &str,
        port: u16,
        flags: u32,
    ) -> lib::CallbackResult<()> {
        dbg!(data, hostname, port, flags);
        Ok(())
    }

    fn pre_connect(&mut self, _context: &mut libfreerdp::Freerdp) -> lib::CallbackResult<()> {
        Ok(())
    }

    fn post_connect(&mut self, instance: &mut lib::Freerdp) -> lib::CallbackResult<()> {
        instance
            .init_gdi(libfreerdp::PixelFormat::Bgr32)
            .map_err(|e| lib::CallbackError::Any(e.to_string()))?;

        let Some(cx) = instance.context() else {
            return Err(libfreerdp::CallbackError::Any("no context".into()));
        };
        let settings = cx.settings();

        self.dispatch(UiEvent::Connected {
            width: settings.get_desktop_width(),
            height: settings.get_desktop_height(),
            fb: self.fb.clone(),
            event_tx: self.event_tx.clone(),
        })
        .map_err(|e| lib::CallbackError::Any(e.to_string()))?;

        Ok(())
    }

    fn post_disconnect(&mut self, instance: &mut lib::Freerdp) -> lib::CallbackResult<()> {
        instance
            .free_gdi()
            .map_err(|e| lib::CallbackError::Any(e.to_string()))?;

        self.dispatch(UiEvent::Disconnected)
            .map_err(|e| lib::CallbackError::Any(e.to_string()))?;

        Ok(())
    }

    fn begin_paint(&mut self, cx: &mut lib::RdpContext) -> libfreerdp::CallbackResult<()> {
        let Some(gdi) = cx.gdi() else { return Ok(()) };

        let Some(mut invalid) = gdi.invalid() else {
            return Ok(());
        };

        invalid.set_null(true);

        Ok(())
    }

    fn end_paint(&mut self, cx: &mut lib::RdpContext) -> libfreerdp::CallbackResult<()> {
        let Some(gdi) = cx.gdi() else { return Ok(()) };

        if gdi.suppress_output() {
            return Ok(());
        }

        let Some(invalid) = gdi.invalid() else {
            return Ok(());
        };

        let x = invalid.x() as usize;
        let y = invalid.y() as usize;
        let w = invalid.w() as usize;
        let h = invalid.h() as usize;
        let stride = gdi.stride() as usize;
        let src = gdi.primary_buffer();

        let mut fb = self
            .fb
            .lock()
            .map_err(|e| lib::CallbackError::Any(e.to_string()))?;
        if fb.width() != gdi.width() as u32 || fb.height() != gdi.height() as u32 {
            fb.resize(gdi.width() as u32, gdi.height() as u32);
        }

        for row in 0..h {
            let src_off = (y + row) * stride + x * 4;
            let dst_off = (y + row) * fb.width() as usize + x;

            let src_row = &src[src_off..src_off + w * 4];
            let dst_row = &mut fb.pixels_mut()[dst_off..dst_off + w];

            for (i, px) in dst_row.iter_mut().enumerate() {
                let b = src_row[i * 4 + 0] as u32;
                let g = src_row[i * 4 + 1] as u32;
                let r = src_row[i * 4 + 2] as u32;
                let a = src_row[i * 4 + 3] as u32;
                *px = (a << 24) + (r << 16) + (g << 8) + (b << 0);
            }
        }

        self.dispatch(UiEvent::Updated)
            .map_err(|e| lib::CallbackError::Any(e.to_string()))?;

        Ok(())
    }

    fn on_channel_connected(&mut self, dvc: lib::Dvc) -> libfreerdp::CallbackResult<()> {
        match dvc {
            lib::Dvc::Disp(disp) => {
                self.shared.borrow_mut().disp = Some(disp);
            }
        };
        Ok(())
    }

    fn on_channel_disconnected(&mut self, _dvc: lib::Dvc) -> libfreerdp::CallbackResult<()> {
        Ok(())
    }
}

#[derive(Debug, Default)]
struct SharedContext {
    disp: Option<lib::DispClientContext>,
}

#[derive(Debug, Default)]
struct MousePosition(u16, u16);

impl MousePosition {
    fn r#move(&mut self, x: u16, y: u16) {
        self.0 = x;
        self.1 = y;
    }
}

fn handle_rdp_event(
    event: RdpEvent,
    cx: &mut lib::RdpContext,
    mouse_pos: &mut MousePosition,
    shared: &Rc<RefCell<SharedContext>>,
) -> Result<ControlFlow<()>> {
    match event {
        crate::rdp::RdpEvent::DisconnectRequested => {
            return Ok(ControlFlow::Break(()));
        }

        crate::rdp::RdpEvent::KeyboardInputed(state, key) => {
            let PhysicalKey::Code(key) = key else {
                return Ok(ControlFlow::Continue(()));
            };
            let Some(code) = to_rdp_scancode(key) else {
                return Ok(ControlFlow::Continue(()));
            };

            let down = state == ElementState::Pressed;

            let mut input = cx.input();
            input.send_keyboard_event(down, false, code)?;
        }

        crate::rdp::RdpEvent::MouseInputed(state, button) => {
            let x = mouse_pos.0;
            let y = mouse_pos.1;

            let mut flags = lib::PtrFlags::empty();
            match state {
                ElementState::Pressed => {
                    flags |= lib::PtrFlags::DOWN;
                }
                ElementState::Released => {}
            };
            match button {
                MouseButton::Left => flags |= lib::PtrFlags::BUTTON1,
                MouseButton::Right => flags |= lib::PtrFlags::BUTTON2,
                MouseButton::Middle => flags |= lib::PtrFlags::BUTTON3,
                _ => {}
            }

            let mut input = cx.input();
            input.send_mouse_event(flags, x, y)?;
        }

        crate::rdp::RdpEvent::CursorMoved(x, y) => {
            mouse_pos.r#move(x as u16, y as u16);
            let x = mouse_pos.0;
            let y = mouse_pos.1;

            let mut input = cx.input();
            input.send_mouse_event(lib::PtrFlags::MOVE, x, y)?;
        }

        RdpEvent::Resized(w, h) => {
            if let Some(disp) = &mut shared.clone().borrow_mut().disp {
                disp.send_monitor_layout(w, h)?;
            };

        }
    };

    Ok(ControlFlow::Continue(()))
}

impl RemoteDesktop {
    pub fn new<S: Into<String>>(
        event_loop_proxy: EventLoopProxy<UiEvent>,
        auth_code_bin: S,
    ) -> RemoteDesktop {
        let (event_tx, event_rx) = super::event::channel();
        Self {
            event_loop_proxy,
            auth_code_bin: auth_code_bin.into(),
            fb: FrameBuffer::new_shared(),
            event_tx,
            event_rx: Some(event_rx),
            shared: Default::default(),
        }
    }

    pub fn run(mut self) -> Result<()> {
        let Some(event_rx) = self.event_rx.take() else {
            return Err(RemoteDesktopError::AlreadyCreated);
        };
        let shared = self.shared.clone();

        let mut settings = lib::Settings::new()?;
        settings.set_server_host_name("suzuki-w11");
        settings.set_server_port(3389);
        settings.set_username("u");
        settings.set_password("123qweASD");
        //settings.set_aad_security(true);

        // jp106
        settings.set_keyboard_layout(0x00000411);
        settings.set_keyboard_type(4);
        settings.set_keyboard_subtype(0);
        settings.set_keyboard_function_key(12);

        let mut cx = lib::new_client_context(self, settings)?;

        cx.connect()?;

        let event_fd = event_rx.as_raw_fd();
        let mut handles = [ptr::null_mut(); 64 + 1];
        handles[0] = lib::fd_to_handle(event_fd);

        let mut mouse_pos = MousePosition::default();

        'event_loop: while !cx.shall_disconnect() {
            let n = cx.get_event_handles(&mut handles[1..]);
            lib::poll(&handles[0..n + 1])?;
            if !cx.check_event_handles() {
                break;
            }

            while let Some(events) = event_rx.recv() {
                for event in events {
                    let r = handle_rdp_event(event, cx.as_mut(), &mut mouse_pos, &shared)?;
                    if r.is_break() {
                        break 'event_loop;
                    }
                }
            }
        }

        drop(cx);

        Ok(())
    }

    fn dispatch(&self, event: UiEvent) -> Result<()> {
        self.event_loop_proxy.send_event(event)?;
        Ok(())
    }
}
