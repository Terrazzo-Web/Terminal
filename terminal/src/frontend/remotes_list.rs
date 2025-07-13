use std::iter::once;
use std::time::Duration;

use terrazzo::autoclone;
use terrazzo::declare_trait_aliias;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::template;
use terrazzo::widgets::cancellable::Cancellable;
use terrazzo::widgets::debounce::DoDebounce as _;
use wasm_bindgen_futures::spawn_local;
use web_sys::MouseEvent;

use self::diagnostics::Instrument as _;
use self::diagnostics::debug;
use crate::api::client_address::ClientAddress;

stylance::import_crate_style!(style, "src/frontend/remotes_list.scss");

#[derive(Clone)]
pub struct RemotesState {
    pub remotes: XSignal<Remotes>,
    show_remotes: Cancellable<()>,
    hide_remotes: Cancellable<Duration>,
}

pub type Remote = Option<ClientAddress>;
pub type Remotes = Option<Vec<ClientAddress>>;

declare_trait_aliias!(
    DisplayRemoteFn,
    Fn(Option<&ClientAddress>) -> (String, Option<&'static str>) + Clone + 'static
);

declare_trait_aliias!(ClickRemoteFn, Fn(MouseEvent, Remote) + Clone + 'static);

impl RemotesState {
    pub fn new() -> Self {
        Self {
            remotes: XSignal::new("remotes", None),
            show_remotes: Cancellable::new(),
            hide_remotes: Duration::from_millis(250).cancellable(),
        }
    }

    pub fn show_remotes_dropdown(
        &self,
        display_remote: impl DisplayRemoteFn,
        click: impl ClickRemoteFn,
    ) -> XElement {
        show_remotes_dropdown(
            display_remote,
            click,
            self.remotes.clone(),
            self.hide_remotes.clone(),
        )
    }

    #[autoclone]
    pub fn mouseenter(&self) -> impl Fn(MouseEvent) + 'static {
        let remote_names_state = self.clone();
        move |_| {
            let Self {
                remotes,
                show_remotes,
                hide_remotes,
            } = &remote_names_state;
            show_remotes.cancel();

            let update_remotes = show_remotes.capture(move |new_remotes| {
                autoclone!(remotes);
                remotes.set(new_remotes)
            });
            hide_remotes.cancel();
            let fetch_remotes = async move {
                let remotes = crate::api::client::remotes::remotes()
                    .await
                    .or_else_throw(|error| format!("Failed to fetch remotes: {error}"));
                if update_remotes(remotes).is_none() {
                    debug!("Updating remotes was canceled");
                }
            };
            spawn_local(fetch_remotes.in_current_span());
        }
    }

    #[autoclone]
    pub fn mouseleave(&self) -> impl Fn(MouseEvent) + 'static {
        let Self {
            remotes,
            hide_remotes,
            ..
        } = self;
        hide_remotes.wrap(move |_| {
            autoclone!(remotes);
            remotes.set(Remotes::None);
        })
    }
}

#[autoclone]
#[html]
#[template(tag = ul)]
fn show_remotes_dropdown(
    display_remote: impl DisplayRemoteFn,
    click: impl ClickRemoteFn,
    #[signal] remotes: Remotes,
    hide_remotes: Cancellable<Duration>,
) -> XElement {
    debug!("Render remote names");
    if let Remotes::Some(remotes) = remotes {
        if !remotes.is_empty() {
            let local_and_remotes = once(None).chain(remotes.into_iter().map(Some));
            let remote_names = local_and_remotes.map(|remote| {
                let (remote_name, remote_class) = display_remote(remote.as_ref());
                li(
                    class = remote_class,
                    "{remote_name}",
                    mouseenter = move |_ev| {
                        autoclone!(hide_remotes);
                        hide_remotes.cancel();
                    },
                    click = move |ev| {
                        autoclone!(click);
                        click(ev, remote.clone())
                    },
                )
            });
            return tag(class = style::remotes_list, remote_names..);
        }
    }
    return tag(style::visibility = "hidden", style::display = "none");
}
