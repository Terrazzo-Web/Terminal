use std::sync::Arc;

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

#[html]
#[template(tag = div)]
pub fn editor(#[signal] content: Option<Arc<str>>) -> XElement {
    tag(
        class = super::style::body,
        textarea(
            after_render = move |element| {
                element.set_text_content(content.as_deref());
            },
        ),
    )
}
