#[cfg(all(feature = "server", not(feature = "client")))]
macro_rules! declare_icon {
    ($file:expr $(,)?) => {
        terrazzo::declare_asset!(concat!("/assets", $file))
    };
}

#[cfg(all(feature = "server", not(feature = "client")))]
type Icon = terrazzo::static_assets::AssetBuilder;

#[cfg(feature = "client")]
macro_rules! declare_icon {
    ($file:expr $(,)?) => {
        concat!("/static", $file)
    };
}

#[cfg(feature = "client")]
type Icon = &'static str;

pub fn add_tab() -> Icon {
    declare_icon!("/icons/plus-square.svg")
}

pub fn menu() -> Icon {
    declare_icon!("/icons/signpost-split.svg")
}

pub fn close_tab() -> Icon {
    declare_icon!("/icons/x-lg.svg")
}

pub fn terminal() -> Icon {
    declare_icon!("/icons/terminal-dash.svg")
}

pub fn text_editor() -> Icon {
    declare_icon!("/icons/layout-text-sidebar-reverse.svg")
}
