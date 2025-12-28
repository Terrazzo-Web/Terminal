use std::sync::OnceLock;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::more_event::MoreEvent as _;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlElement;
use web_sys::HtmlInputElement;

use self::diagnostics::Instrument as _;
use self::diagnostics::info;
use self::diagnostics::warn;
use crate::assets::icons;
use crate::frontend::menu::app;
use crate::frontend::remotes::Remote;
use crate::state::app::App;

stylance::import_style!(style, "login.scss");

#[autoclone]
#[html]
#[template]
pub fn login(#[signal] mut logged_in: LoggedInStatus, remote: XSignal<Remote>) -> XElement {
    match logged_in {
        LoggedInStatus::Login => div(
            key = "app",
            div(move |t| show_app(t, app(), remote.clone())),
        ),
        LoggedInStatus::Logout => div(
            key = "login",
            class = style::login,
            img(class = style::key_icon, src = icons::key_icon()),
            input(
                r#type = "password",
                after_render = |password: &Element| {
                    let password: &HtmlElement = password.dyn_ref().or_throw("password");
                    let () = password.focus().or_throw("password focus");
                },
                change = move |ev: web_sys::Event| {
                    let Ok(password): Result<HtmlInputElement, _> = ev
                        .current_target_element("The password field")
                        .map_err(|error| warn!("{error}"))
                    else {
                        return;
                    };

                    let login_task = async move {
                        autoclone!(logged_in_mut);
                        match crate::api::client::login::login(Some(&password.value())).await {
                            Ok(()) => logged_in_mut.set(LoggedInStatus::Login),
                            Err(error) => warn!("{error}"),
                        }
                    };
                    spawn_local(login_task.in_current_span());
                },
            ),
        ),
        LoggedInStatus::Unknown => {
            let login_task = async move {
                autoclone!(logged_in_mut);
                match crate::api::client::login::login(None).await {
                    Ok(()) => logged_in_mut.set(LoggedInStatus::Login),
                    Err(error) => {
                        logged_in_mut.set(LoggedInStatus::Logout);
                        info!("Authentication is required: {error}")
                    }
                }
            };
            spawn_local(login_task.in_current_span());
            div(key = "login-pending", class = style::login)
        }
    }
}

pub fn logged_in() -> XSignal<LoggedInStatus> {
    static LOGGED_IN: OnceLock<XSignal<LoggedInStatus>> = OnceLock::new();
    LOGGED_IN
        .get_or_init(|| XSignal::new("logged-in", LoggedInStatus::Unknown))
        .clone()
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum LoggedInStatus {
    Login,
    Logout,

    #[default]
    Unknown,
}

#[html]
#[template]
fn show_app(#[signal] app: App, remote: XSignal<Remote>) -> XElement {
    match app {
        #[cfg(feature = "terminal")]
        App::Terminal => {
            drop(remote);
            div(|t| crate::terminal::terminals(t))
        }
        #[cfg(feature = "text-editor")]
        App::TextEditor => div(move |t| crate::text_editor::ui::text_editor(t, remote.clone())),
        #[cfg(feature = "converter")]
        App::Converter => div(move |t| crate::converter::ui::converter(t, remote.clone())),
        #[cfg(feature = "port-forward")]
        App::PortForward => div(move |t| crate::portforward::ui::port_forward(t, remote.clone())),
    }
}
