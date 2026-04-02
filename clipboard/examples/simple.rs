use std::sync::mpsc;

use clipboard::{Clipboard, ClipboardEvent};

fn main() {
    let (tx, rx) = mpsc::channel::<()>();

    let mut clipboard = Clipboard::new(move |event| {
        match event {
            ClipboardEvent::SelectionCleared => {}
            ClipboardEvent::SelectionOwnerChanged(formats) => {
                println!("{formats:?}");
                tx.send(()).ok();
            }
            ClipboardEvent::GetTextReply(text) => {
                println!("{text:?}");
                tx.send(()).ok();
            }
        }
    })
    .unwrap();
    let _handle = clipboard.start().unwrap();

    loop {
        rx.recv().ok();
        clipboard.request_get_text().unwrap();
        rx.recv().ok();
        clipboard.request_set_text("OK").unwrap();
    }

    // handle.join().unwrap().unwrap();
}
