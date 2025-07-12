use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::cancellable::Cancellable;
use terrazzo::widgets::debounce::DoDebounce as _;
use web_sys::MouseEvent;

use self::diagnostics::Instrument as _;
use crate::assets::icons;
use crate::frontend::remotes::Remote;
use crate::state::app;
use crate::state::app::App;

stylance::import_crate_style!(style, "src/frontend/menu.scss");

pub fn before_menu() -> MutexGuard<'static, Option<Box<dyn FnOnce() + Send>>> {
    static BEFORE_MENU: Mutex<Option<Box<dyn FnOnce() + Send>>> = Mutex::new(None);
    BEFORE_MENU.lock().unwrap()
}

#[autoclone]
#[html]
#[template(tag = div)]
pub fn menu() -> XElement {
    let hide_menu = Duration::from_millis(500).cancellable();
    div(
        class = style::menu,
        div(
            class = style::menu_inner,
            img(class = style::menu_icon, src = icons::menu()),
            mouseover = move |_: MouseEvent| {
                autoclone!(hide_menu);
                if let Some(f) = before_menu().take() {
                    f()
                };
                hide_menu.cancel();
                show_menu().set(true);
            },
        ),
        mouseout = hide_menu
            .clone()
            .wrap(|_: MouseEvent| show_menu().set(false)),
        menu_items(show_menu(), hide_menu.clone()),
    )
}

#[autoclone]
#[html]
#[template(tag = ul)]
fn menu_items(#[signal] mut show_menu: bool, hide_menu: Cancellable<Duration>) -> XElement {
    if show_menu {
        let mut items = vec![];
        #[cfg(feature = "terminal")]
        items.push(menu_item(
            App::Terminal,
            app(),
            show_menu_mut.clone(),
            hide_menu.clone(),
        ));
        items.push(menu_item(
            App::TextEditor,
            app(),
            show_menu_mut.clone(),
            hide_menu.clone(),
        ));
        tag(
            class = style::menu_items,
            mouseover = move |_: MouseEvent| {
                autoclone!(hide_menu, show_menu_mut);
                hide_menu.cancel();
                show_menu_mut.set(true);
            },
            items..,
        )
    } else {
        tag(style::visibility = "hidden", style::display = "none")
    }
}

#[autoclone]
#[html]
#[template(tag = li)]
fn menu_item(
    app: App,
    #[signal] mut selected_app: App,
    show_menu_mut: MutableSignal<bool>,
    hide_menu: Cancellable<Duration>,
) -> XElement {
    tag(
        img(class = style::app_icon, src = app.icon()),
        "{app}",
        class = (selected_app == app).then_some(style::active),
        click = move |_| {
            autoclone!(selected_app_mut);
            let batch = Batch::use_batch("select-app");
            hide_menu.cancel();
            show_menu_mut.set(false);
            selected_app_mut.set(app);
            drop(batch);
        },
    )
}

impl App {
    pub fn icon(&self) -> icons::Icon {
        match self {
            #[cfg(feature = "terminal")]
            App::Terminal => icons::terminal(),
            App::TextEditor => icons::text_editor(),
        }
    }
}

#[autoclone]
pub fn app() -> XSignal<App> {
    static STATIC: OnceLock<XSignal<App>> = OnceLock::new();
    STATIC
        .get_or_init(|| {
            let app = XSignal::new("app", App::default());
            let load_app_task = async move {
                autoclone!(app);
                // The client address is set per app, not globally.
                let address: Remote = None;
                if let Ok(p) = app::state::get(address).await {
                    app.set(p);
                }
            };
            wasm_bindgen_futures::spawn_local(load_app_task.in_current_span());
            static CONSUMER: OnceLock<Consumers> = OnceLock::new();
            let _ = CONSUMER.set(app.add_subscriber(|app| {
                let store_app_task = async move {
                    // The client address is set per app, not globally.
                    let address: Remote = None;
                    let _ = app::state::set(address, app).await;
                };
                wasm_bindgen_futures::spawn_local(store_app_task.in_current_span())
            }));
            app
        })
        .clone()
}

fn show_menu() -> XSignal<bool> {
    static STATIC: OnceLock<XSignal<bool>> = OnceLock::new();
    STATIC
        .get_or_init(|| XSignal::new("show-menu", false))
        .clone()
}
