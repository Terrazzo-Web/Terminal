//! Represents a flying cookie in the game

use std::cell::Cell;
use std::iter::once;
use std::rc::Rc;

use terrazzo::autoclone;
use terrazzo::envelope;
use terrazzo::html;
use terrazzo::owned_closure::XOwnedClosure;
use terrazzo::prelude::*;
use terrazzo::template;
use web_sys::js_sys::Math::random;

use super::position::Position;
use super::size::Size;
use super::state::Game;

stylance::import_style!(style, "cookie.scss");

#[template(tag = img, key = c.id.to_string())]
#[html]
pub fn cookie(c: CookiePtr) -> XElement {
    tag(
        class = style::cookie,
        style::top %= cookie_style::top(c.position.clone()),
        style::left %= cookie_style::left(c.position.clone()),
        style::width %= cookie_style::width(c.size.clone()),
        style::height %= cookie_style::height(c.size.clone()),
        src = "/static/game/cookie.jpg",
    )
}

mod cookie_style {
    use terrazzo::prelude::*;
    use terrazzo::template;

    use crate::game::position::Position;
    use crate::game::size::Size;

    #[template(wrap = true)]
    pub fn top(#[signal] mut position: Position) -> XAttributeValue {
        format!("{}px", position.top)
    }

    #[template(wrap = true)]
    pub fn left(#[signal] mut position: Position) -> XAttributeValue {
        format!("{}px", position.left)
    }

    #[template(wrap = true)]
    pub fn width(#[signal] mut size: Size) -> XAttributeValue {
        format!("{}px", size.x)
    }

    #[template(wrap = true)]
    pub fn height(#[signal] mut size: Size) -> XAttributeValue {
        format!("{}px", size.y)
    }
}

#[envelope(ptr = std::rc::Rc)]
#[derive(Debug)]
pub struct Cookie {
    id: usize,
    position: XSignal<Position>,
    size: XSignal<Size>,
    speed: f64,
}

impl CookiePtr {
    pub fn new(game: &Game) -> Self {
        use std::sync::atomic::AtomicUsize;
        use std::sync::atomic::Ordering::SeqCst;
        static NEXT: AtomicUsize = AtomicUsize::new(0);

        let id = NEXT.fetch_add(1, SeqCst);

        let window_size = game.window_size.get_value_untracked();
        let size_f = 1. / rand_f64(15., 35.);
        let c = Self::from(Cookie {
            id,
            position: XSignal::new(
                "position",
                Position {
                    top: random() * (window_size.y - (window_size * size_f).y) as f64,
                    left: window_size.x as f64,
                },
            ),
            size: game
                .window_size
                .view("cookie_size", move |window_size| *window_size * size_f),
            speed: rand_f64(10., 100.),
        });
        make_cookie_move(game.clone(), c.clone());
        return c;
    }
}

#[autoclone]
fn make_cookie_move(game: Game, c: CookiePtr) {
    let handle = Rc::new(Cell::new(None));
    let closure = XOwnedClosure::new(|self_drop| {
        move || {
            autoclone!(game, c, handle);
            let create_new_cookie = c.position.update(|p| {
                let left = p.left - c.speed;
                let create_new_cookie = left < -c.size.get_value_untracked().x as f64;
                if create_new_cookie {
                    game.window
                        .clear_interval_with_handle(handle.get().or_throw("clear_interval"));
                    self_drop().or_throw("self_drop");
                }
                Some(Position { left, ..*p }).and_return(create_new_cookie)
            });
            if create_new_cookie {
                game.cookies.update(move |cookies| {
                    autoclone!(game, c);
                    Some(
                        cookies
                            .iter()
                            .filter(|cc| cc.id != c.id)
                            .cloned()
                            .chain(once(CookiePtr::new(&game)))
                            .collect(),
                    )
                });
            }
        }
    });
    let closure = closure.as_function();
    handle.set(Some(
        game.window
            .set_interval_with_callback_and_timeout_and_arguments_0(&closure, 1000 / 30)
            .or_throw("set_interval"),
    ));
}

fn rand_f64(from: f64, to: f64) -> f64 {
    random() * (to - from) + from
}
