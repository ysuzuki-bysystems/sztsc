use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::RawKeyEvent;
use winit::event_loop::ControlFlow;
use winit::window::Window;

use crate::event::UiEvent;
use crate::frame_buffer::SharedFrameBuffer;
use crate::rdp::{RdpEvent, RdpEventSender};

#[derive(Debug)]
struct State {
    window: Rc<Window>,
    _context: Context<Rc<Window>>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    fb: SharedFrameBuffer,
    rdp_event_tx: RdpEventSender,
}

#[derive(Default, Debug)]
pub struct App {
    state: Option<State>,
}

impl ApplicationHandler<UiEvent> for App {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // nop
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        event_loop.set_control_flow(ControlFlow::Wait);

        let Some(state) = &mut self.state else {
            return;
        };
        if state.window.id() != window_id {
            return;
        }

        match event {
            winit::event::WindowEvent::CursorMoved {
                position: PhysicalPosition { x, y },
                ..
            } => {
                state.rdp_event_tx.send(RdpEvent::CursorMoved(x, y));
            }

            winit::event::WindowEvent::MouseInput {
                state: mouse_state,
                button,
                ..
            } => {
                state
                    .rdp_event_tx
                    .send(RdpEvent::MouseInputed(mouse_state, button));
            }

            winit::event::WindowEvent::CloseRequested => {
                state.rdp_event_tx.send(RdpEvent::DisconnectRequested);
            }

            winit::event::WindowEvent::RedrawRequested => {
                let fb = state.fb.lock().unwrap();
                if fb.width() == 0 || fb.height() == 0 {
                    return;
                }

                state
                    .surface
                    .resize(
                        NonZeroU32::new(fb.width() as u32).unwrap(),
                        NonZeroU32::new(fb.height() as u32).unwrap(),
                    )
                    .unwrap();

                let mut buffer = state.surface.buffer_mut().unwrap();
                buffer.copy_from_slice(&fb.pixels());
                buffer.present().unwrap();
            }

            _ => {}
        }
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        event_loop.set_control_flow(ControlFlow::Wait);

        let Some(state) = &mut self.state else {
            return;
        };

        match event {
            winit::event::DeviceEvent::Key(RawKeyEvent {
                state: key_state,
                physical_key,
            }) => {
                state
                    .rdp_event_tx
                    .send(RdpEvent::KeyboardInputed(key_state, physical_key));
            }

            _ => {}
        };
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UiEvent) {
        match event {
            UiEvent::Connected {
                width,
                height,
                fb,
                event_tx,
            } => {
                if self.state.is_some() {
                    return;
                }

                let attrs =
                    Window::default_attributes().with_inner_size(PhysicalSize::new(width, height));
                let window = event_loop.create_window(attrs).unwrap();
                let window = Rc::new(window);

                let context = Context::new(window.clone()).unwrap();
                let surface = Surface::new(&context, window.clone()).unwrap();

                self.state = Some(State {
                    window,
                    _context: context,
                    surface,
                    fb,
                    rdp_event_tx: event_tx,
                });
            }

            UiEvent::Disconnected => {
                self.state = None;
            }

            UiEvent::Updated => {
                let Some(state) = &self.state else {
                    return;
                };
                state.window.request_redraw();
            }

            UiEvent::Done => event_loop.exit(),
        }
    }
}
