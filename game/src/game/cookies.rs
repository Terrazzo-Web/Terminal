use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use super::cookie::CookiePtr;
use super::cookie::cookie;

stylance::import_style!(style, "cookies.scss");

#[template(tag = div)]
#[html]
pub fn show_cookies(#[signal] cookies: Vec<CookiePtr>) -> XElement {
    let cookies = cookies.into_iter().map(cookie);
    div(class = style::cookies, cookies..)
}
