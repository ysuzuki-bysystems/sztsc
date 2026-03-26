use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::ControlFlow;
use winit::window::Window;

use crate::event::UiEvent;
use crate::frame_buffer::SharedFrameBuffer;
use crate::rdp::{RdpEvent, RdpEventSender};
use super::inhibitor::InhibitState;
use super::inhibitor::WaylandInhibitor;

#[derive(Debug)]
struct State {
    window: Rc<Window>,
    _context: Context<Rc<Window>>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    fb: SharedFrameBuffer,
    rdp_event_tx: RdpEventSender,
    inhibitor: Option<WaylandInhibitor<InhibitState>>,
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
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                state
                    .rdp_event_tx
                    .send(RdpEvent::KeyboardInputed(event.state, event.physical_key));
            }

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
                window.set_ime_allowed(false);
                let window = Rc::new(window);
                let inhibitor = match WaylandInhibitor::try_new(&window, InhibitState::default()) {
                    Ok(inhibitor) => inhibitor,
                    Err(err) => {
                        eprint!("{err}");
                        None
                    },
                };

                let context = Context::new(window.clone()).unwrap();
                let surface = Surface::new(&context, window.clone()).unwrap();

                self.state = Some(State {
                    window,
                    _context: context,
                    surface,
                    fb,
                    rdp_event_tx: event_tx,
                    inhibitor,
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

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let Some(state) = &mut self.state else {
            return;
        };
        let Some(inhibitor) = &mut state.inhibitor else {
            return;
        };
        if let Err(err) = inhibitor.dispatch_pending() {
            eprint!("{err}");
        };
    }
}
