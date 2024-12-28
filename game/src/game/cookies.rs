use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use super::cookie::cookie;
use super::cookie::Cookie;
use super::state::Game;

stylance::import_crate_style!(style, "src/game/cookies.scss");

#[template(tag = div)]
#[html]
pub fn show_cookies(game: Game, #[signal] cookies: Vec<Cookie>) -> XElement {
    let cookies = cookies.into_iter().map(move |c| cookie(game.clone(), c));
    div(class = style::cookies, cookies..)
}
