use std::{env, thread};

use winit::event_loop;

mod event;
mod frame_buffer;
mod rdp;
mod ui;

fn main() -> anyhow::Result<()> {
    let event_loop = event_loop::EventLoop::<event::UiEvent>::with_user_event().build()?;

    let auth_code_bin = env::var("SZTSC_AUTHCODE_BIN").unwrap_or("sztsc-authcode".into());
    let proxy = event_loop.create_proxy();
    let _ = thread::spawn(move || {
        let rdp = rdp::RemoteDesktop::new(proxy, auth_code_bin);
        rdp::run(rdp)
    });

    let mut app = ui::App::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
