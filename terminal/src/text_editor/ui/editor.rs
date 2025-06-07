use std::sync::Arc;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::SeqCst;

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

use super::code_mirror::CodeMirrorJs;

#[html]
#[template(tag = div)]
pub fn editor(#[signal] content: Option<Arc<str>>) -> XElement {
    static NEXT: AtomicI32 = AtomicI32::new(1);
    let key = format!("editor-{}", NEXT.fetch_add(1, SeqCst));
    tag(
        class = super::style::body,
        div(
            key = key,
            after_render = move |element| {
                if let Some(content) = &content {
                    drop(CodeMirrorJs::new(element, content.as_ref().into()))
                }
            },
        ),
    )
}
