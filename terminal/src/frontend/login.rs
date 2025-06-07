use std::cell::OnceCell;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::more_event::MoreEvent as _;
use tracing::info;
use tracing::warn;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlElement;
use web_sys::HtmlInputElement;

use super::menu::App;
use super::menu::app;
use crate::assets::icons;
use crate::terminal::terminals;
use crate::text_editor::ui::text_editor;

stylance::import_crate_style!(style, "src/frontend/login.scss");

#[autoclone]
#[html]
#[template]
pub fn login(#[signal] mut logged_in: LoggedInStatus) -> XElement {
    match logged_in {
        LoggedInStatus::Login => div(key = "app", div(|t| show_app(t, app()))),
        LoggedInStatus::Logout => div(
            key = "login",
            class = style::login,
            img(class = style::key_icon, src = icons::key_icon()),
            input(
                r#type = "password",
                after_render = |password: Element| {
                    let password: HtmlElement = password.dyn_into().or_throw("password");
                    let () = password.focus().or_throw("password focus");
                },
                change = move |ev: web_sys::Event| {
                    let Ok(password): Result<HtmlInputElement, _> = ev
                        .current_target_element("The password field")
                        .map_err(|error| warn!("{error}"))
                    else {
                        return;
                    };

                    spawn_local(async move {
                        autoclone!(logged_in_mut);
                        match crate::api::client::login::login(Some(&password.value())).await {
                            Ok(()) => logged_in_mut.set(LoggedInStatus::Login),
                            Err(error) => warn!("{error}"),
                        }
                    });
                },
            ),
        ),
        LoggedInStatus::Unknown => {
            spawn_local(async move {
                autoclone!(logged_in_mut);
                match crate::api::client::login::login(None).await {
                    Ok(()) => logged_in_mut.set(LoggedInStatus::Login),
                    Err(error) => {
                        logged_in_mut.set(LoggedInStatus::Logout);
                        info!("Authentication is required: {error}")
                    }
                }
            });
            div(key = "login-pending", class = style::login)
        }
    }
}

pub fn logged_in() -> XSignal<LoggedInStatus> {
    static LOGGED_IN: LoggedIn = LoggedIn(OnceCell::new());
    LOGGED_IN
        .0
        .get_or_init(|| XSignal::new("logged-in", LoggedInStatus::Unknown))
        .clone()
}

struct LoggedIn(OnceCell<XSignal<LoggedInStatus>>);
unsafe impl Sync for LoggedIn {}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum LoggedInStatus {
    Login,
    Logout,

    #[default]
    Unknown,
}

#[html]
#[template]
fn show_app(#[signal] app: App) -> XElement {
    match app {
        App::Terminal => div(|t| terminals(t)),
        App::TextEditor => div(|t| text_editor(t)),
    }
}
