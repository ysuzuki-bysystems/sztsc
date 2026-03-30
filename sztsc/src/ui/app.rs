use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::ElementState;
use winit::event_loop::ControlFlow;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window};

use super::inhibitor::InhibitState;
use super::inhibitor::WaylandInhibitor;
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
    inhibitor: Option<WaylandInhibitor<InhibitState>>,
}

#[derive(Debug)]
struct ResizeState {
    pending: PhysicalSize<u32>,
    deadline: Instant,
}

#[derive(Debug, Default)]
struct FullScreenHotKeyState {
    ctrl_right: bool,
    shift_right: bool,
    arrow_up: bool,
}

impl FullScreenHotKeyState {
    fn is_all_after_update(&mut self, state: ElementState, key: PhysicalKey) -> bool {
        match key {
            PhysicalKey::Code(key) => match key {
                KeyCode::ControlRight => self.ctrl_right = state == ElementState::Pressed,
                KeyCode::ShiftRight => self.shift_right = state == ElementState::Pressed,
                KeyCode::ArrowUp => self.arrow_up = state == ElementState::Pressed,
                _ => return false,
            },
            _ => return false,
        }
        self.ctrl_right && self.shift_right && self.arrow_up
    }
}

#[derive(Default, Debug)]
pub struct App {
    state: Option<State>,
    resize_state: Option<ResizeState>,
    full_screen_hot_key_state: FullScreenHotKeyState,
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
                if self
                    .full_screen_hot_key_state
                    .is_all_after_update(event.state, event.physical_key)
                {
                    if state.window.fullscreen().is_some() {
                        state.window.set_fullscreen(None);
                    } else {
                        state
                            .window
                            .set_fullscreen(Some(Fullscreen::Borderless(None)));
                    }

                    return;
                }

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

            winit::event::WindowEvent::Resized(size) => {
                const DEBOUNCE: Duration = Duration::from_millis(500);
                let deadline = Instant::now() + DEBOUNCE;
                self.resize_state = Some(ResizeState {
                    pending: size,
                    deadline,
                });
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
                    }
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

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let Some(state) = &mut self.state else {
            return;
        };

        if let Some(resize) = self.resize_state.take() {
            if Instant::now() < resize.deadline {
                event_loop.set_control_flow(ControlFlow::WaitUntil(resize.deadline));
                self.resize_state = Some(resize);
            } else {
                state.rdp_event_tx.send(RdpEvent::Resized(
                    resize.pending.width,
                    resize.pending.height,
                ));
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        };

        let Some(inhibitor) = &mut state.inhibitor else {
            return;
        };
        if let Err(err) = inhibitor.dispatch_pending() {
            eprint!("{err}");
        };
    }
}
