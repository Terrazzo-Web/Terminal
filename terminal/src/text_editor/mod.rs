mod autocomplete;
mod code_mirror;
mod editor;
mod fsio;
mod path_selector;
mod state;
mod synchronized_state;
pub mod ui;

#[cfg(feature = "client")]
stylance::import_crate_style!(style, "src/text_editor/text_editor.scss");
