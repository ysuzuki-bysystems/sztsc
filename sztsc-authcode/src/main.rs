use std::env;

use gtk::traits::BoxExt;
use gtk::traits::WidgetExt;
use tao::dpi::LogicalPosition;
use tao::event::Event;
use tao::event::WindowEvent;
use tao::event_loop::ControlFlow;
use tao::event_loop::EventLoopBuilder;
use tao::event_loop::EventLoopProxy;
use tao::event_loop::EventLoopWindowTarget;
use tao::platform::unix::WindowExtUnix;
use tao::window::Window;
use tao::window::WindowBuilder;
use tao::window::WindowId;
use url::Url;
use wry::Rect;
use wry::WebContext;
use wry::WebView;
use wry::WebViewBuilder;
use wry::WebViewBuilderExtUnix;

#[derive(Debug, Clone)]
enum AppState {
    Begin,
    LoggedIn,
}

#[derive(Debug)]
enum UserEvent {
    OnPageLoadStarted(String),
    StateChanged(AppState),
    Succeeded(String),
    Failure(String),
    Canceled,
}

struct Handler {
    state: Option<AppState>,
    url: Url,
    redirect_uri: String,
    dispatcher: EventLoopProxy<UserEvent>,
    window: Window,
    webview: WebView,
}

const LOGIN_URL: &'static str = "https://login.microsoftonline.com/";

impl Handler {
    fn handle_user_event(
        &mut self,
        event: UserEvent,
        _target: &EventLoopWindowTarget<UserEvent>,
        ctrl: &mut ControlFlow,
    ) -> anyhow::Result<()> {
        match event {
            UserEvent::OnPageLoadStarted(url) => {
                let Some(state) = &self.state else {
                    return Ok(());
                };

                match state {
                    AppState::Begin => {
                        let login_origin = Url::parse(LOGIN_URL)?.origin();
                        let origin = Url::parse(&url)?.origin();
                        if origin != login_origin {
                            self.dispatcher
                                .send_event(UserEvent::StateChanged(AppState::LoggedIn))?;
                        }
                    }

                    AppState::LoggedIn => {
                        let url = Url::parse(&url)?;
                        let mut url_without_query = url.clone();
                        url_without_query.set_query(None);
                        url_without_query.set_fragment(None);
                        if url_without_query.as_str() != self.redirect_uri {
                            return Ok(());
                        }

                        for (key, val) in url.query_pairs() {
                            match key.as_ref() {
                                "code" => {
                                    self.dispatcher
                                        .send_event(UserEvent::Succeeded(val.to_string()))?;
                                }

                                "error" => {
                                    self.dispatcher
                                        .send_event(UserEvent::Failure(val.to_string()))?;
                                }

                                _ => {}
                            }
                        }
                    }
                }

                *ctrl = ControlFlow::Wait;
            }

            UserEvent::StateChanged(state) => {
                match state {
                    AppState::Begin => {
                        self.webview.load_url(LOGIN_URL)?;
                    }

                    AppState::LoggedIn => {
                        self.webview.load_url(self.url.as_str())?;
                    }
                }
                self.state = Some(state.clone());
                *ctrl = ControlFlow::Wait;
            }

            UserEvent::Succeeded(code) => {
                println!("{code}");
                *ctrl = ControlFlow::Exit;
            }

            UserEvent::Failure(message) => {
                eprintln!("{message}");
                *ctrl = ControlFlow::ExitWithCode(-1);
            }

            UserEvent::Canceled => {
                eprintln!("Canceled");
                *ctrl = ControlFlow::ExitWithCode(-1);
            }
        }

        Ok(())
    }

    fn handle_window_event(
        &mut self,
        window_id: WindowId,
        event: WindowEvent,
        _target: &EventLoopWindowTarget<UserEvent>,
        ctrl: &mut ControlFlow,
    ) -> anyhow::Result<()> {
        match event {
            WindowEvent::Resized(_) => {
                *ctrl = ControlFlow::Wait;

                if window_id != self.window.id() {
                    return Ok(());
                }

                let size = self.window.inner_size();
                self.webview.set_bounds(Rect {
                    position: LogicalPosition::<u32>::default().into(),
                    size: size.into(),
                })?;
            }

            WindowEvent::CloseRequested => {
                if window_id != self.window.id() {
                    *ctrl = ControlFlow::Wait;
                    return Ok(());
                }

                self.dispatcher.send_event(UserEvent::Canceled)?;
                *ctrl = ControlFlow::Wait;
            }

            _ => {
                *ctrl = ControlFlow::Wait;
            }
        }

        Ok(())
    }

    fn handle(
        &mut self,
        event: Event<UserEvent>,
        target: &EventLoopWindowTarget<UserEvent>,
        ctrl: &mut ControlFlow,
    ) -> anyhow::Result<()> {
        match event {
            Event::WindowEvent {
                window_id, event, ..
            } => {
                self.handle_window_event(window_id, event, target, ctrl)?;
            }

            Event::UserEvent(event) => {
                self.handle_user_event(event, target, ctrl)?;
            }

            _ => {
                *ctrl = ControlFlow::Wait;
            }
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = env::args();
    let Some(url) = args.skip(1).next() else {
        anyhow::bail!("Usage: %prog [url]");
    };

    let url = Url::parse(&url)?;
    let Some(redirect_uri) = url.query_pairs().find_map(|(key, val)| {
        if key != "redirect_uri" {
            return None;
        }

        Some(val.to_string())
    }) else {
        anyhow::bail!("Missing query: redirect_uri");
    };

    gtk::init()?;

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let dispatcher = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title(env!("CARGO_PKG_NAME"))
        .build(&event_loop)?;
    let vbox = window.default_vbox().unwrap();
    let fixed = gtk::Fixed::new();
    fixed.show_all();
    vbox.pack_start(&fixed, true, true, 0);

    let size = window.inner_size();

    // TODO xdg_???/
    let dirs = xdg::BaseDirectories::with_prefix("sztsc");
    let dir = dirs.create_data_directory("webkit")?;
    let mut cx = WebContext::new(Some(dir));
    let webview = WebViewBuilder::new_with_web_context(&mut cx)
        .with_bounds(Rect {
            position: LogicalPosition::<u32>::default().into(),
            size: size.into(),
        })
        .with_on_page_load_handler(move |event, current| {
            if matches!(event, wry::PageLoadEvent::Started) {
                return;
            }
            dispatcher
                .send_event(UserEvent::OnPageLoadStarted(current))
                .unwrap();
        })
        .build_gtk(&fixed)?;

    let dispatcher = event_loop.create_proxy();
    dispatcher.send_event(UserEvent::StateChanged(AppState::Begin))?;
    let mut handler = Handler {
        state: None,
        url,
        redirect_uri,
        dispatcher,
        window,
        webview,
    };

    event_loop.run(move |event, target, ctrl| {
        let Err(err) = handler.handle(event, target, ctrl) else {
            return;
        };

        eprintln!("{err}");
        *ctrl = ControlFlow::ExitWithCode(-1);
    });
}
