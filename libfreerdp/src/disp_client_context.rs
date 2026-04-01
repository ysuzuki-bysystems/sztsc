use std::ptr;

use super::Result;
use super::lib;

#[derive(Debug)]
pub struct DispClientContext {
    raw: ptr::NonNull<lib::DispClientContext>,
}

impl DispClientContext {
    pub(super) fn from_raw(raw: ptr::NonNull<lib::DispClientContext>) -> Self {
        Self { raw }
    }

    pub fn send_monitor_layout(&mut self, w: u32, h: u32) -> Result<()> {
        let mut mon = lib::DISPLAY_CONTROL_MONITOR_LAYOUT {
            Flags: 0x00000001, // DISPLAY_CONTROL_MONITOR_PRIMARY
            Left: 0,
            Top: 0,
            Width: w,
            Height: h,
            PhysicalWidth: w,  // FIXME
            PhysicalHeight: h, // FIXME
            Orientation: 0,
            DesktopScaleFactor: 100,
            DeviceScaleFactor: 100,
        };

        let ptr = unsafe { self.raw.as_mut() };
        let Some(send_monitor_layout) = ptr.SendMonitorLayout else {
            return Ok(());
        };
        let r = unsafe { send_monitor_layout(self.raw.as_ptr(), 1, &mut mon as *mut _) };
        if r != 0 {
            todo!()
        }

        Ok(())
    }
}
