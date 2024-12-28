use terrazzo_client::prelude::*;
use terrazzo_macro::html;
use terrazzo_macro::template;
use web_sys::MouseEvent;

stylance::import_crate_style!(style, "src/widgets/link.scss");

#[html]
#[template]
pub fn link<C, CI>(
    click: impl Fn(MouseEvent) + Clone + 'static,
    content: impl FnOnce() -> CI + Clone + 'static,
) -> XElement
where
    XNode: From<C>,
    CI: IntoIterator<Item = C>,
{
    span! {
        class = style::link,
        click = click,
        content()..
    }
}
