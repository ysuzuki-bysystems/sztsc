use thiserror::Error;
use wayland_backend::client::Backend;
use wayland_backend::client::InvalidId;
use wayland_backend::client::ObjectId;
use wayland_backend::client::WaylandError;
use wayland_client::Connection;
use wayland_client::Dispatch;
use wayland_client::DispatchError;
use wayland_client::Proxy;
use wayland_client::globals::BindError;
use wayland_client::globals::GlobalError;
use wayland_client::globals::GlobalListContents;
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::__interfaces::WL_SURFACE_INTERFACE;
use wayland_client::protocol::wl_registry;
use wayland_client::protocol::wl_seat;
use wayland_client::protocol::wl_surface;
use wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1;
use wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibitor_v1;
use wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1;
use winit::raw_window_handle::HandleError;
use winit::raw_window_handle::HasDisplayHandle;
use winit::raw_window_handle::HasWindowHandle;
use winit::raw_window_handle::RawDisplayHandle;
use winit::raw_window_handle::RawWindowHandle;
use winit::window::Window;

#[derive(Debug, Default)]
pub(super) struct InhibitState {
    active: bool,
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for InhibitState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for InhibitState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_seat::WlSeat,
        _event: <wl_seat::WlSeat as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpKeyboardShortcutsInhibitManagerV1, ()> for InhibitState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpKeyboardShortcutsInhibitManagerV1,
        _event: <ZwpKeyboardShortcutsInhibitManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpKeyboardShortcutsInhibitorV1, ()> for InhibitState {
    fn event(
        state: &mut Self,
        _proxy: &ZwpKeyboardShortcutsInhibitorV1,
        event: <ZwpKeyboardShortcutsInhibitorV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            zwp_keyboard_shortcuts_inhibitor_v1::Event::Active => {
                state.active = true;
                eprintln!("shortcut inhibitor: active");
            }
            zwp_keyboard_shortcuts_inhibitor_v1::Event::Inactive => {
                state.active = false;
                eprintln!("shortcut inhibitor: inactive");
            }
            _ => {}
        }
    }
}

#[derive(Debug, Error)]
pub(super) enum WaylandInhibitorError {
    #[error("{0}")]
    RawDisplayHandle(#[from] HandleError),

    #[error("{0}")]
    Global(#[from] GlobalError),

    #[error("{0}")]
    Bind(#[from] BindError),

    #[error("{0}")]
    InvalidId(#[from] InvalidId),

    #[error("{0}")]
    Wayland(#[from] WaylandError),

    #[error("{0}")]
    Dispatch(#[from] DispatchError),
}

/// Dangerous Implement.
#[derive(Debug)]
pub(super) struct WaylandInhibitor<S>
where
    S: Dispatch<wl_registry::WlRegistry, GlobalListContents>
        + Dispatch<wl_seat::WlSeat, ()>
        + Dispatch<ZwpKeyboardShortcutsInhibitManagerV1, ()>
        + Dispatch<ZwpKeyboardShortcutsInhibitorV1, ()>
        + 'static,
{
    //conn: Connection,
    event_queue: wayland_client::EventQueue<S>,
    //qhandle: QueueHandle<S>,
    //seat: wl_seat::WlSeat,
    //manager: ZwpKeyboardShortcutsInhibitManagerV1,
    inhibitor: ZwpKeyboardShortcutsInhibitorV1,
    state: S,
}

impl<S> Drop for WaylandInhibitor<S>
where
    S: Dispatch<wl_registry::WlRegistry, GlobalListContents>
        + Dispatch<wl_seat::WlSeat, ()>
        + Dispatch<ZwpKeyboardShortcutsInhibitManagerV1, ()>
        + Dispatch<ZwpKeyboardShortcutsInhibitorV1, ()>
        + 'static,
{
    fn drop(&mut self) {
        self.inhibitor.destroy();
        self.event_queue.dispatch_pending(&mut self.state).unwrap();
        self.event_queue.flush().unwrap();
    }
}

impl<S> WaylandInhibitor<S>
where
    S: Dispatch<wl_registry::WlRegistry, GlobalListContents>
        + Dispatch<wl_seat::WlSeat, ()>
        + Dispatch<ZwpKeyboardShortcutsInhibitManagerV1, ()>
        + Dispatch<ZwpKeyboardShortcutsInhibitorV1, ()>
        + 'static,
{
    pub(super) fn try_new(
        window: &Window,
        state: S,
    ) -> Result<Option<Self>, WaylandInhibitorError> {
        let RawDisplayHandle::Wayland(display) = window.display_handle()?.as_raw() else {
            return Ok(None);
        };
        let RawWindowHandle::Wayland(surface) = window.window_handle()?.as_raw() else {
            return Ok(None);
        };

        // wl_display*
        let wl_display = display.display.as_ptr();
        // wl_surface*
        let wl_surface = surface.surface.as_ptr();

        let backend = unsafe { Backend::from_foreign_display(wl_display.cast()) };
        let conn = Connection::from_backend(backend);

        let (globals, event_queue) = registry_queue_init::<S>(&conn)?;
        let qhandle = event_queue.handle();

        let seat = globals.bind::<wl_seat::WlSeat, _, _>(&qhandle, 1..=9, ())?;
        let manager =
            globals.bind::<ZwpKeyboardShortcutsInhibitManagerV1, _, _>(&qhandle, 1..=1, ())?;

        let surface_id = unsafe { ObjectId::from_ptr(&WL_SURFACE_INTERFACE, wl_surface.cast()) }?;
        let wl_surface = wl_surface::WlSurface::from_id(&conn, surface_id)?;

        let inhibitor = manager.inhibit_shortcuts(&wl_surface, &seat, &qhandle, ());

        Ok(Some(Self {
            //conn,
            event_queue,
            //qhandle,
            //seat,
            //manager,
            inhibitor,
            state,
        }))
    }

    pub(super) fn dispatch_pending(&mut self) -> Result<(), WaylandInhibitorError> {
        self.event_queue.dispatch_pending(&mut self.state)?;
        self.event_queue.flush()?;

        Ok(())
    }
}
