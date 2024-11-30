use autoclone_macro::autoclone;
use terrazzo_client::owned_closure::XOwnedClosure;
use terrazzo_client::prelude::*;
use terrazzo_macro::html;
use terrazzo_macro::template;
use web_sys::window;

stylance::import_crate_style!(style, "src/game.scss");

#[autoclone]
#[template]
#[html]
pub fn run() -> XElement {
    let window = window().unwrap();
    let cookies = [1., 5., 20.]
        .into_iter()
        .enumerate()
        .map(|(i, d)| {
            let position = XSignal::new(
                "cookie-pos",
                Position {
                    right: 0.,
                    top: 50. + 220. * i as f64,
                },
            );
            let closure = XOwnedClosure::new(|self_drop| {
                move || {
                    autoclone!(position);
                    let _self_drop = &self_drop;
                    position.update(|p| {
                        Some(Position {
                            right: p.right + d,
                            ..*p
                        })
                    });
                }
            });
            let closure = closure.as_function().unwrap();
            window
                .set_interval_with_callback_and_timeout_and_arguments_0(&closure, 10)
                .unwrap();
            return img(
                class = style::cookie,
                style %= move |t| {
                    autoclone!(position);
                    position_style(t, position.clone())
                },
                src = "/static/game/cookie.jpg",
            );
            // return img(move |t: XTemplate| moving_cookie(t, position.clone()));
        })
        .collect::<Vec<_>>();
    div(
        class = style::game,
        img(class = style::game_board, src = "/static/game/picture.jpg"),
        cookies..,
    )
}

#[template]
#[html]
pub fn position_style(#[signal] cookie_style: Position) -> XAttributeValue {
    format!(
        "top: {}px; right: {}px;",
        cookie_style.top, cookie_style.right
    )
    .into()
}

#[derive(Clone, Copy, Debug)]
struct Position {
    right: f64,
    top: f64,
}
