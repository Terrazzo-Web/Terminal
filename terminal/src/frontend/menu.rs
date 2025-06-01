use std::cell::OnceCell;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use crate::assets::icons;

#[html]
#[template(tag = ul)]
pub fn show_menu() -> XElement {
    tag(
        show_menu_item(App::Terminal, app()),
        show_menu_item(App::TextEditor, app()),
    )
}

#[autoclone]
#[html]
#[template(tag = li)]
pub fn show_menu_item(app: App, #[signal] mut selected_app: App) -> XElement {
    tag(
        img(src = app.icon()),
        "{app}",
        class = (selected_app == app).then_some("active"),
        click = move |_| {
            autoclone!(selected_app_mut);
            selected_app_mut.set(app);
        },
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum App {
    Terminal,
    TextEditor,
}

impl std::fmt::Display for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            App::Terminal => "Terminal",
            App::TextEditor => "Text editor",
        }
        .fmt(f)
    }
}

impl App {
    #[allow(unused)]
    pub fn icon(&self) -> &'static str {
        match self {
            App::Terminal => icons::terminal(),
            App::TextEditor => icons::text_editor(),
        }
    }
}

pub fn app() -> XSignal<App> {
    static APP: CurrentApp = CurrentApp(OnceCell::new());
    APP.0
        .get_or_init(|| XSignal::new("app", App::Terminal))
        .clone()
}

struct CurrentApp(OnceCell<XSignal<App>>);
unsafe impl Sync for CurrentApp {}
