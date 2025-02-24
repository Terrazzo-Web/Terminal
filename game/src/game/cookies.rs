use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use super::cookie::Cookie;
use super::cookie::cookie;

stylance::import_crate_style!(style, "src/game/cookies.scss");

#[template(tag = div)]
#[html]
pub fn show_cookies(#[signal] cookies: Vec<Cookie>) -> XElement {
    let cookies = cookies.into_iter().map(cookie);
    div(class = style::cookies, cookies..)
}
