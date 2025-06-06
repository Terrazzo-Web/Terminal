use std::sync::Arc;

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;

const JS: &str = r#"
    const [
        { EditorView },
        { basicSetup },
        { javascript }
    ] = await Promise.all([
        import("https://esm.sh/@codemirror/view@latest"),
        import("https://esm.sh/@codemirror/basic-setup@latest"),
        import("https://esm.sh/@codemirror/lang-javascript@latest"),
    ]);

    new EditorView({
        doc: "console.log('Hello, dynamically loaded CodeMirror!');",
        extensions: [basicSetup, javascript()],
        parent: document.getElementById("editor"),
    });
"#;

#[html]
#[template(tag = div)]
pub fn editor(#[signal] content: Option<Arc<str>>) -> XElement {
    tag(
        class = super::style::body,
        pre(after_render = move |element| {
            element.set_text_content(content.as_deref());
        }),
        div(id = "editor"),
        script(r#type = "module", "{JS}"),
    )
}
