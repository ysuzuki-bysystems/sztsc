use core::fmt;
use std::io;
use std::os::fd::AsRawFd;
use std::os::fd::RawFd;
use std::sync::mpsc;

use eventfd::EfdFlags;
use eventfd::EventFD;
use winit::event::ElementState;
use winit::event::MouseButton;
use winit::keyboard::PhysicalKey;

#[derive(Debug, Clone)]
pub enum RdpEvent {
    DisconnectRequested,
    // FIXME type leaked
    KeyboardInputed(ElementState, PhysicalKey),
    // FIXME type leaked
    MouseInputed(ElementState, MouseButton),
    CursorMoved(f64, f64),
}

pub(super) struct RdpEventReciever {
    fd: EventFD,
    channel: mpsc::Receiver<RdpEvent>,
}

impl fmt::Debug for RdpEventReciever {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RdpEventReciever")
            .field("fd", &self.fd.as_raw_fd())
            .field("channel", &self.channel)
            .finish()
    }
}

impl AsRawFd for RdpEventReciever {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl RdpEventReciever {
    pub(super) fn recv(&self) -> Option<Vec<RdpEvent>> {
        let n = match self.fd.read() {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                return None;
            }
            r => r.unwrap(),
        };

        let mut results = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let item = self.channel.try_recv().unwrap();
            results.push(item);
        }
        Some(results)
    }
}

#[derive(Clone)]
pub struct RdpEventSender {
    fd: EventFD,
    channel: mpsc::Sender<RdpEvent>,
}

impl fmt::Debug for RdpEventSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RdpEventSender")
            .field("fd", &self.fd.as_raw_fd())
            .field("channel", &self.channel)
            .finish()
    }
}

impl RdpEventSender {
    pub fn send(&self, val: RdpEvent) {
        self.channel.send(val).unwrap();
        self.fd.write(1).unwrap();
    }
}

pub(super) fn channel() -> (RdpEventSender, RdpEventReciever) {
    let fd = EventFD::new(0, EfdFlags::EFD_CLOEXEC | EfdFlags::EFD_NONBLOCK).unwrap();
    let (tx, rx) = mpsc::channel();

    let sender = RdpEventSender {
        fd: fd.clone(),
        channel: tx,
    };
    let reciever = RdpEventReciever { fd, channel: rx };

    (sender, reciever)
}
