//! Represents a flying cookie in the game

use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

use autoclone_macro::autoclone;
use terrazzo_client::owned_closure::XOwnedClosure;
use terrazzo_client::prelude::*;
use terrazzo_macro::html;
use terrazzo_macro::template;
use web_sys::js_sys::Math::random;

use super::position::Position;
use super::size::Size;
use super::state::Game;

stylance::import_crate_style!(style, "src/game/cookie.scss");

#[autoclone]
#[template(tag = img)]
#[html]
pub fn cookie(game: Game, c: Cookie) -> XElement {
    make_cookie_move(game.clone(), c.clone());
    img(
        class = style::cookie,
        style %= move |t| {
            autoclone!(c);
            cookie_style(t, c.position.clone(), c.size.clone())
        },
        src = "/static/game/cookie.jpg",
    )
}

#[autoclone]
fn make_cookie_move(game: Game, c: Cookie) {
    let handle = Rc::new(Cell::new(None));
    let closure = XOwnedClosure::new(|self_drop| {
        move || {
            autoclone!(game, c, handle);
            let create_new_cookie = c.position.update(|p| {
                let left = p.left - 30.;
                let create_new_cookie = left < -c.size.get_value_untracked().x as f64;
                if create_new_cookie {
                    game.window
                        .clear_interval_with_handle(handle.get().unwrap());
                    self_drop().unwrap();
                }
                Some(Position { left, ..*p }).and_return(create_new_cookie)
            });
            if create_new_cookie {
                let c2 = vec![Cookie::new(&game, 1. / 20.), Cookie::new(&game, 1. / 20.)];
                game.cookies.update(|_| Some(c2));
            }
        }
    });
    let closure = closure.as_function().unwrap();
    handle.set(Some(
        game.window
            .set_interval_with_callback_and_timeout_and_arguments_0(&closure, 1000 / 30)
            .unwrap(),
    ));
}

#[template]
fn cookie_style(#[signal] mut position: Position, #[signal] mut size: Size) -> XAttributeValue {
    let _position_mut = position_mut;
    let _size_mut = size_mut;
    format!(
        "top: {top}px; left: {left}px; width: {width}px; height: {height}px",
        top = position.top,
        left = position.left,
        width = size.x,
        height = size.y,
    )
    .into()
}

#[derive(Clone, Debug)]
pub struct Cookie {
    inner: Rc<CookieInner>,
}

#[derive(Debug)]
pub struct CookieInner {
    #[expect(unused)]
    id: usize,
    position: XSignal<Position>,
    size: XSignal<Size>,
}

impl Cookie {
    pub fn new(game: &Game, size_f: f64) -> Self {
        use std::sync::atomic::AtomicUsize;
        use std::sync::atomic::Ordering::SeqCst;
        static NEXT: AtomicUsize = AtomicUsize::new(0);

        let id = NEXT.fetch_add(1, SeqCst);

        let window_size = game.window_size.get_value_untracked();
        let size = game
            .window_size
            .view("cookie_size", move |window_size| *window_size * size_f);
        let position = XSignal::new(
            "position",
            Position {
                top: random() * (window_size.y - (window_size * size_f).y) as f64,
                left: window_size.x as f64,
            },
        );
        Self {
            inner: Rc::new(CookieInner { id, position, size }),
        }
    }
}

impl Deref for Cookie {
    type Target = CookieInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
