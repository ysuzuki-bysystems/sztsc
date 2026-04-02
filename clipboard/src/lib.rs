use std::string::FromUtf8Error;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;

use thiserror::Error;
use x11rb::CURRENT_TIME;
use x11rb::connection::Connection;
use x11rb::errors::ConnectError;
use x11rb::errors::ConnectionError;
use x11rb::errors::ReplyError;
use x11rb::errors::ReplyOrIdError;
use x11rb::protocol::Event;
use x11rb::protocol::xfixes::ConnectionExt as _;
use x11rb::protocol::xfixes::SelectionEventMask;
use x11rb::protocol::xproto::AtomEnum;
use x11rb::protocol::xproto::CLIENT_MESSAGE_EVENT;
use x11rb::protocol::xproto::ClientMessageEvent;
use x11rb::protocol::xproto::ConnectionExt as _;
use x11rb::protocol::xproto::CreateWindowAux;
use x11rb::protocol::xproto::EventMask;
use x11rb::protocol::xproto::PropMode;
use x11rb::protocol::xproto::SELECTION_NOTIFY_EVENT;
use x11rb::protocol::xproto::SelectionNotifyEvent;
use x11rb::protocol::xproto::WindowClass;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("{0}")]
    Connect(#[from] ConnectError),

    #[error("{0}")]
    ConnectionError(#[from] ConnectionError),

    #[error("{0}")]
    GenerateId(#[from] ReplyOrIdError),

    #[error("{0}")]
    Reply(#[from] ReplyError),

    #[error("XFIXES not exists.")]
    XfixesNotPresent,

    #[error("already started.")]
    AlreadyStarted,

    #[error("{0}")]
    FromUtf8(#[from] FromUtf8Error),

    #[error("not started.")]
    NotStarted,
}

#[derive(Debug, Clone)]
struct Atoms {
    clipboard: u32,
    targets: u32,
    property: u32,
    utf8_string: u32,

    get_text: u32,
    set_text: u32,
}

impl Atoms {
    fn new(conn: &RustConnection) -> Result<Self, ClipboardError> {
        Ok(Self {
            clipboard: conn.intern_atom(false, b"CLIPBOARD")?.reply()?.atom,
            targets: conn.intern_atom(false, b"TARGETS")?.reply()?.atom,
            property: conn
                .intern_atom(false, b"APP_CLIPBOARD_TARGETS")?
                .reply()?
                .atom,
            utf8_string: conn.intern_atom(false, b"UTF8_STRING")?.reply()?.atom,

            get_text: conn.intern_atom(false, b"APP_GET_TEXT")?.reply()?.atom,
            set_text: conn.intern_atom(false, b"APP_SET_TEXT")?.reply()?.atom,
        })
    }
}

#[derive(Debug)]
pub enum ClipboardEvent {
    SelectionCleared,
    SelectionOwnerChanged(Vec<String>),
    GetTextReply(String),
}

#[derive(Debug)]
enum Request {
    GetText,
    SetText,
}

pub struct Clipboard<F: FnMut(ClipboardEvent) + Send + 'static> {
    // for Request
    conn: RustConnection,
    window: Option<u32>,
    atoms: Option<Atoms>,
    callback: Option<F>,
    memory: Arc<Mutex<Vec<u8>>>,
}

impl<F: FnMut(ClipboardEvent) + Send + 'static> Clipboard<F> {
    pub fn new(callback: F) -> Result<Self, ClipboardError> {
        let (conn, _) = x11rb::connect(None)?;

        Ok(Self {
            conn,
            window: None,
            atoms: None,
            callback: Some(callback),
            memory: Arc::new(Mutex::new(vec![])),
        })
    }

    pub fn start(&mut self) -> Result<JoinHandle<Result<(), ClipboardError>>, ClipboardError> {
        if self.window.is_some() {
            return Err(ClipboardError::AlreadyStarted);
        }
        let Some(mut callback) = self.callback.take() else {
            return Err(ClipboardError::AlreadyStarted);
        };
        let memory = self.memory.clone();

        let (conn, screen_num) = x11rb::connect(None)?;
        let screen = &conn.setup().roots[screen_num];
        let window = conn.generate_id()?;
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            screen.root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )?;

        let xfixes = conn.query_extension(b"XFIXES")?.reply()?;
        if !xfixes.present {
            return Err(ClipboardError::XfixesNotPresent);
        }

        let _ = conn.xfixes_query_version(5, 0)?;

        let atoms = Atoms::new(&conn)?;

        conn.xfixes_select_selection_input(
            window,
            atoms.clipboard,
            SelectionEventMask::SET_SELECTION_OWNER
                | SelectionEventMask::SELECTION_WINDOW_DESTROY
                | SelectionEventMask::SELECTION_CLIENT_CLOSE,
        )?;
        conn.flush()?;

        self.window = Some(window);
        self.atoms = Some(atoms.clone());

        Ok(thread::spawn(move || {
            loop {
                let event = conn.wait_for_event()?;

                match event {
                    Event::XfixesSelectionNotify(notify) => {
                        if notify.selection != atoms.clipboard {
                            continue;
                        }

                        // selection owner changed

                        if notify.owner == x11rb::NONE {
                            // Cleared
                            callback(ClipboardEvent::SelectionCleared);
                            continue;
                        }

                        conn.delete_property(window, atoms.property).ok();

                        conn.convert_selection(
                            window,
                            atoms.clipboard,
                            atoms.targets,
                            atoms.property,
                            CURRENT_TIME,
                        )?;
                        conn.flush()?;

                        continue;
                    }

                    Event::SelectionNotify(notify) => {
                        if notify.requestor != window {
                            continue;
                        }
                        if notify.selection != atoms.clipboard {
                            continue;
                        }

                        if notify.target == atoms.targets {
                            // Request for formats.
                            if notify.property != atoms.property {
                                panic!(); // TODO
                            }

                            let reply = conn
                                .get_property(
                                    false,
                                    window,
                                    atoms.property,
                                    AtomEnum::ATOM,
                                    0,
                                    u32::MAX,
                                )?
                                .reply()?;
                            if reply.type_ != AtomEnum::ATOM.into() || reply.format != 32 {
                                continue;
                                //panic!()
                            }
                            let results = reply.value32().unwrap();

                            conn.delete_property(window, atoms.property)?;
                            conn.flush()?;

                            let mut formats = vec![];
                            for atom in results {
                                let name = conn.get_atom_name(atom)?.reply()?;
                                let name = String::from_utf8(name.name)?;
                                match name.as_str() {
                                    "TARGETS" | "TIMESTAMP" | "MULTIPLE" | "SAVE_TARGETS" => {
                                        continue;
                                    }
                                    _ => {}
                                };
                                formats.push(name);
                            }

                            callback(ClipboardEvent::SelectionOwnerChanged(formats));
                            continue;
                        }

                        if notify.target == atoms.utf8_string {
                            let reply = conn
                                .get_property(
                                    false,
                                    window,
                                    atoms.property,
                                    AtomEnum::ANY,
                                    0,
                                    u32::MAX,
                                )?
                                .reply()?;
                            if reply.type_ != atoms.utf8_string {
                                panic!()
                            }
                            let value = String::from_utf8(reply.value)?;
                            callback(ClipboardEvent::GetTextReply(value));

                            continue;
                        }

                        continue;
                    }

                    Event::SelectionRequest(req) => {
                        if req.selection != atoms.clipboard {
                            continue;
                        }

                        let memory = memory.lock().unwrap();
                        let property = if req.property == x11rb::NONE { req.target } else { req.property };
                        if req.target == atoms.targets {
                            let supported = [atoms.utf8_string];
                            conn.change_property32(
                                PropMode::REPLACE,
                                req.requestor,
                                property,
                                AtomEnum::ATOM,
                                &supported[..],
                            )?;
                            let notify = SelectionNotifyEvent {
                                response_type: SELECTION_NOTIFY_EVENT,
                                sequence: 0,
                                time: req.time,
                                requestor: req.requestor,
                                selection: req.selection,
                                target: req.target,
                                property: property,
                            };
                            conn.send_event(false, req.requestor, EventMask::NO_EVENT, notify)?;
                            continue;
                        }

                        if req.target == atoms.utf8_string {
                            conn.change_property8(
                                PropMode::REPLACE,
                                req.requestor,
                                property,
                                req.target,
                                &memory,
                            )?;
                            let notify = SelectionNotifyEvent {
                                response_type: SELECTION_NOTIFY_EVENT,
                                sequence: 0,
                                time: req.time,
                                requestor: req.requestor,
                                selection: req.selection,
                                target: req.target,
                                property: property,
                            };
                            conn.send_event(false, req.requestor, EventMask::NO_EVENT, notify)?;
                            conn.flush()?;
                            continue;
                        }
                    }

                    Event::ClientMessage(msg) => {
                        if msg.window != window {
                            continue;
                        }

                        if msg.type_ == atoms.get_text {
                            conn.convert_selection(
                                window,
                                atoms.clipboard,
                                atoms.utf8_string,
                                atoms.property,
                                CURRENT_TIME,
                            )?;
                            conn.flush()?;
                            continue;
                        }

                        if msg.type_ == atoms.set_text {
                            conn.set_selection_owner(window, atoms.clipboard, CURRENT_TIME)?;
                            conn.flush()?;

                            let owner = conn.get_selection_owner(atoms.clipboard)?.reply()?.owner;
                            if owner != window {
                                panic!()
                            }
                            continue;
                        }
                    }

                    _ => continue,
                }
            }
        }))
    }

    pub fn request_get_text(&self) -> Result<(), ClipboardError> {
        self.dispatch(Request::GetText)
    }

    pub fn request_set_text(&self, val: &str) -> Result<(), ClipboardError> {
        let data = val.as_bytes().to_vec();
        *self.memory.lock().unwrap() = data;

        self.dispatch(Request::SetText)
    }

    fn dispatch(&self, request: Request) -> Result<(), ClipboardError> {
        let (Some(window), Some(atoms)) = (self.window, &self.atoms) else {
            return Err(ClipboardError::NotStarted);
        };

        let type_ = match request {
            Request::GetText => atoms.get_text,
            Request::SetText => atoms.set_text,
        };

        let event = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window,
            type_,
            data: [0, 0, 0, 0, 0].into(),
        };

        self.conn
            .send_event(false, window, EventMask::NO_EVENT, event)?;
        self.conn.flush()?;
        Ok(())
    }
}
