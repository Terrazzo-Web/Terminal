#![cfg(feature = "client")]

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::resize_event::ResizeEvent;
use tracing::info;
use web_sys::window;

use self::cookie::Cookie;
use self::cookies::show_cookies;
use self::state::Game;

mod cookie;
mod cookies;
mod position;
mod size;
mod state;

stylance::import_crate_style!(style, "src/game.scss");

#[autoclone]
#[template]
#[html]
pub fn run(main: Element) -> XElement {
    let window = window().unwrap();
    ResizeEvent::set_up(&window);
    let game = Game::new(window, main);
    div(
        class = style::game,
        img(
            class = style::board,
            src = "/static/game/picture.jpg",
            load = move |_: web_sys::Event| {
                autoclone!(game);
                info!("Loading the game");
                ResizeEvent::signal().force(());
                game.cookies
                    .update(|_| Some((0..20).map(|_| Cookie::new(&game)).collect()));
            },
        ),
        show_cookies(game.cookies.clone()),
    )
}
