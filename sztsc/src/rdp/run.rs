use winit::event_loop::EventLoopProxy;

use super::RemoteDesktop;
use crate::event::UiEvent;

struct Finally {
    event_loop_proxy: EventLoopProxy<UiEvent>,
}

impl Finally {
    fn new(event_loop_proxy: &EventLoopProxy<UiEvent>) -> Self {
        Self {
            event_loop_proxy: event_loop_proxy.clone(),
        }
    }
}

impl Drop for Finally {
    fn drop(&mut self) {
        self.event_loop_proxy.send_event(UiEvent::Done).ok();
    }
}

pub fn run(rdp: RemoteDesktop) {
    let finally = Finally::new(&rdp.event_loop_proxy);

    rdp.run().expect("Failed to run.");

    drop(finally);
}
