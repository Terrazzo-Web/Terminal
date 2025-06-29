#![cfg(feature = "client")]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

use scopeguard::guard;
use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use self::diagnostics::debug;
use super::code_mirror::CodeMirrorJs;
use super::synchronized_state::SynchronizedState;
use crate::text_editor::fsio;
use crate::text_editor::fsio::ui::store_file;
use crate::text_editor::notify::EventKind;
use crate::text_editor::notify::NotifyResponse;
use crate::text_editor::style;
use crate::text_editor::ui::EditorState;
use crate::text_editor::ui::TextEditor;

#[autoclone]
#[html]
#[template(tag = div)]
pub fn editor(
    text_editor: Arc<TextEditor>,
    editor_state: EditorState,
    content: Arc<str>,
) -> XElement {
    let EditorState {
        base_path,
        file_path,
        ..
    } = editor_state;

    let writing = Arc::new(AtomicBool::new(false));
    let on_change: Closure<dyn FnMut(JsValue)> = Closure::new(move |content: JsValue| {
        autoclone!(text_editor, base_path, file_path, writing);
        let Some(content) = content.as_string() else {
            debug!("Changed content is not a string");
            return;
        };
        let write = async move {
            autoclone!(text_editor, base_path, file_path, writing);
            writing.store(true, SeqCst);
            let before = guard((), move |()| writing.store(false, SeqCst));
            let after = SynchronizedState::enqueue(text_editor.synchronized_state.clone());
            let () = store_file(
                text_editor.remote.clone(),
                base_path,
                file_path,
                content,
                before,
                after,
            )
            .await;
        };
        wasm_bindgen_futures::spawn_local(write);
    });

    let code_mirror = Arc::new(Mutex::new(None));

    let notify_registration = text_editor.notify_service.add_handler(notify_handler(
        &text_editor,
        &code_mirror,
        &base_path,
        &file_path,
        &writing,
    ));

    tag(
        class = style::editor,
        after_render = move |element| {
            autoclone!(base_path, file_path);
            let _moved = notify_registration.clone();
            *code_mirror.lock().unwrap() = Some(CodeMirrorJs::new(
                element,
                content.as_ref().into(),
                &on_change,
                base_path.to_string(),
                PathBuf::from(base_path.as_ref())
                    .join(file_path.as_ref())
                    .to_string_lossy()
                    .to_string(),
            ));
        },
    )
}

#[autoclone]
fn notify_handler(
    text_editor: &Arc<TextEditor>,
    code_mirror: &Arc<Mutex<Option<CodeMirrorJs>>>,
    base_path: &Arc<str>,
    file_path: &Arc<str>,
    writing: &Arc<AtomicBool>,
) -> impl Fn(&NotifyResponse) + 'static {
    move |response| {
        autoclone!(text_editor, code_mirror, base_path, file_path, writing);
        match response.kind {
            EventKind::Create | EventKind::Modify => {
                if writing.load(SeqCst) {
                    return;
                }
                spawn_local(async move {
                    autoclone!(text_editor, code_mirror, base_path, file_path);
                    match fsio::ui::load_file(text_editor.remote.clone(), base_path, file_path)
                        .await
                    {
                        Ok(Some(fsio::File::TextFile {
                            metadata: _,
                            content,
                        })) => {
                            let Some(code_mirror) = &*code_mirror.lock().unwrap() else {
                                return;
                            };
                            code_mirror.set_content(content.to_string());
                        }
                        Ok(None) => (), // TODO: remove file
                        Ok(Some(fsio::File::Folder { .. })) => (),
                        Ok(Some(fsio::File::Error { .. })) => (),
                        Err(_) => todo!(),
                    };
                });
            }
            EventKind::Delete | EventKind::Error => text_editor.file_path.set(""),
        }
    }
}
