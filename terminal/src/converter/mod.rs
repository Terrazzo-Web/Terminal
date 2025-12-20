#![cfg(feature = "converter")]

use std::sync::Arc;

use crate::state::make_state::make_state;

mod api;
mod conversion_tabs;
mod service;
pub mod ui;

make_state!(content_state, Arc<str>);
