#![cfg(feature = "converter")]

use std::sync::Arc;

use crate::state::make_state::make_state;

mod api;
mod service;
mod tabs;
pub mod ui;

make_state!(content_state, Arc<str>);
