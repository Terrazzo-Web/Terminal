mod autocomplete;
mod code_mirror;
mod editor;
pub mod file_path;
mod folder;
mod fsio;
mod manager;
pub mod notify;
mod path_selector;
mod remotes;
mod rust_lang;
mod side;
mod state;
mod synchronized_state;
pub mod ui;

#[cfg(feature = "client")]
stylance::import_crate_style!(style, "src/text_editor/text_editor.scss");
