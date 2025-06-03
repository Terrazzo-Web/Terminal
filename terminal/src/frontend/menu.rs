use std::cell::OnceCell;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::cancellable::Cancellable;
use terrazzo::widgets::debounce::DoDebounce as _;
use web_sys::MouseEvent;

use crate::assets::icons;

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
                before_menu().take().map(|f| f());
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
        tag(
            class = style::menu_items,
            mouseover = move |_: MouseEvent| {
                autoclone!(hide_menu, show_menu_mut);
                hide_menu.cancel();
                show_menu_mut.set(true);
            },
            menu_item(
                App::Terminal,
                app(),
                show_menu_mut.clone(),
                hide_menu.clone(),
            ),
            menu_item(
                App::TextEditor,
                app(),
                show_menu_mut.clone(),
                hide_menu.clone(),
            ),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum App {
    Terminal,
}

impl std::fmt::Display for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            App::Terminal => "Terminal",
        }
        .fmt(f)
    }
}

impl App {
    #[allow(unused)]
    pub fn icon(&self) -> &'static str {
        match self {
            App::Terminal => icons::terminal(),
        }
    }
}

pub fn app() -> XSignal<App> {
    struct Static(OnceCell<XSignal<App>>);
    unsafe impl Sync for Static {}

    static STATIC: Static = Static(OnceCell::new());
    STATIC
        .0
        .get_or_init(|| XSignal::new("app", App::Terminal))
        .clone()
}

fn show_menu() -> XSignal<bool> {
    struct Static(OnceCell<XSignal<bool>>);
    unsafe impl Sync for Static {}

    static STATIC: Static = Static(OnceCell::new());
    STATIC
        .0
        .get_or_init(|| XSignal::new("show-menu", false))
        .clone()
}
