use std::ops::Deref;
use std::rc::Rc;

use terrazzo_client::prelude::*;
use terrazzo_common::widgets::resize_event::ResizeEvent;
use web_sys::Window;

use super::cookie::Cookie;
use super::size::Size;

#[derive(Clone)]
pub struct Game {
    inner: Rc<GameInner>,
}

impl Game {
    pub fn new(window: Window, element: Element) -> Self {
        Self {
            inner: Rc::new(GameInner {
                window,
                cookies: XSignal::new("cookies", vec![]),
                window_size: ResizeEvent::signal().view("window_size", move |()| Size {
                    x: element.client_width(),
                    y: element.client_height(),
                }),
            }),
        }
    }
}

pub struct GameInner {
    pub window: Window,
    pub cookies: XSignal<Vec<Cookie>>,
    pub window_size: XSignal<Size>,
}

impl Deref for Game {
    type Target = GameInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
