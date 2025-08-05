#[cfg(all(feature = "server", not(feature = "client")))]
macro_rules! declare_icon {
    ($name:ident, $file:expr; $($predicate:tt)*) => {
        #[cfg($($predicate)*)]
        declare_icon!($name, $file);
    };
    ($name:ident, $file:expr) => {
        pub fn $name() -> Icon {
            terrazzo::declare_asset!(concat!("/assets", $file))
        }
    };
}

#[cfg(all(feature = "server", not(feature = "client")))]
pub type Icon = terrazzo::static_assets::AssetBuilder;

#[cfg(feature = "client")]
macro_rules! declare_icon {
    ($name:ident, $file:expr; $($predicate:tt)*) => {
        #[cfg($($predicate)*)]
        declare_icon!($name, $file);
    };
    ($name:ident, $file:expr) => {
        pub fn $name() -> Icon {
            concat!("/static", $file)
        }
    };
}

#[cfg(feature = "client")]
pub type Icon = &'static str;

declare_icon!(add_port_forward,"/icons/add-port-forward.svg"; feature = "port-forward");
declare_icon!(add_tab, "/icons/plus-square.svg"; feature = "terminal");
declare_icon!(chevron_double_right, "/icons/chevron-double-right.svg"; feature = "text-editor");
declare_icon!(close_tab, "/icons/x-lg.svg"; any(feature = "terminal", feature = "text-editor"));
declare_icon!(converter, "/icons/regex.svg"; feature = "converter");
declare_icon!(done, "/icons/done.svg"; feature = "text-editor");
declare_icon!(file, "/icons/file-earmark-text.svg"; feature = "text-editor");
declare_icon!(folder, "/icons/folder2-open.svg"; feature = "text-editor");
declare_icon!(hub, "/icons/hub.svg"; feature = "port-forward");
declare_icon!(key_icon, "/icons/key.svg");
declare_icon!(loading, "/icons/loading2.svg"; feature = "text-editor");
declare_icon!(menu, "/icons/signpost-split.svg");
declare_icon!(port_forward_loading,"/icons/port-forward-loading.svg"; feature = "port-forward");
declare_icon!(port_forward_pending,"/icons/port-forward-pending.svg"; feature = "port-forward");
declare_icon!(port_forward_synchronized,"/icons/port-forward-synchronized.svg"; feature = "port-forward");
declare_icon!(slash, "/icons/slash.svg"; feature = "text-editor");
declare_icon!(terminal, "/icons/terminal-dash.svg"; feature = "terminal");
declare_icon!(text_editor, "/icons/layout-text-sidebar-reverse.svg"; feature = "text-editor");
declare_icon!(trash, "/icons/trash3.svg"; feature = "port-forward");
