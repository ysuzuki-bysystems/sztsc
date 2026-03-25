use crate::frame_buffer::SharedFrameBuffer;
use crate::rdp::RdpEventSender;

#[derive(Debug)]
pub enum UiEvent {
    Connected {
        width: u32,
        height: u32,
        fb: SharedFrameBuffer,
        event_tx: RdpEventSender,
    },
    Disconnected,
    Updated,
    Done,
}
