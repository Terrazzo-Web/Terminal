use std::rc::Rc;

use autoclone_macro::autoclone;
use terrazzo_client::prelude::*;
use terrazzo_macro::html;
use terrazzo_macro::template;

stylance::import_crate_style!(style, "src/widgets/tabs.scss");

const DRAG_KEY: &str = "id";

pub trait TabsDescriptor: Clone + 'static {
    type State: TabsState<TabDescriptor = Self::TabDescriptor>;
    type TabDescriptor: TabDescriptor<State = Self::State>;

    fn tab_descriptors(&self) -> &[Self::TabDescriptor];

    fn before_titles(&self, _state: &Self::State) -> impl IntoIterator<Item = impl Into<XNode>> {
        let empty: [XNode; 0] = [];
        return empty;
    }

    fn after_titles(&self, _state: &Self::State) -> impl IntoIterator<Item = impl Into<XNode>> {
        let empty: [XNode; 0] = [];
        return empty;
    }
}

pub trait TabsState: Clone + 'static {
    type TabDescriptor: TabDescriptor<State = Self>;
    fn move_tab(&self, after_tab: Option<Self::TabDescriptor>, moved_tab_key: String);
}

pub trait TabDescriptor: Clone + 'static {
    type State: Clone + 'static;
    fn key(&self) -> XString;
    fn title(&self, state: &Self::State) -> impl Into<XNode>;
    fn item(&self, state: &Self::State) -> impl Into<XNode>;
    fn selected(&self, state: &Self::State) -> XSignal<bool>;
}

#[derive(Default)]
pub struct TabsOptions {
    pub tabs_class: Option<XString>,
    pub titles_class: Option<XString>,
    pub title_class: Option<XString>,
    pub items_class: Option<XString>,
    pub item_class: Option<XString>,
    pub selected_class: Option<XString>,
    pub show_separator: Option<XString>,
    pub hide_separator: Option<XString>,
    pub hover_separator: Option<XString>,
}

/// ```
/// <tabs>
///     <titles>
///         <title> ... </title>
///         <title selected> ... </title>
///         <title> ... </title>
///     </titles>
///     <items>
///         <item> ... </item>
///         <item selected> ... </item>
///         <item> ... </item>
///     </items>
/// </tabs>
/// ```
#[template]
#[html]
#[autoclone]
pub fn tabs<T: TabsDescriptor>(tabs_descriptor: T, state: T::State, tabs_options: Rc<TabsOptions>) {
    fn merge_class(
        base_class: &'static str,
        override_class: &Option<impl std::fmt::Display>,
    ) -> XString {
        override_class
            .as_ref()
            .map(|override_class| format!("{} {}", base_class, override_class).into())
            .unwrap_or(base_class.into())
    }

    let TabsOptions {
        tabs_class,
        titles_class,
        title_class,
        items_class,
        item_class,
        selected_class,
        show_separator,
        hide_separator,
        hover_separator,
    } = &*tabs_options;

    let tab_descriptors = || tabs_descriptor.tab_descriptors().iter();
    let tabs_class = merge_class(style::tabs, tabs_class);
    let titles_class = merge_class(style::titles, titles_class);
    let title_class = merge_class(style::title, title_class);
    let items_class = merge_class(style::items, items_class);
    let item_class = merge_class(style::item, item_class);
    let selected_class = merge_class(style::selected, selected_class);
    let show_separator = merge_class(style::show_separator, show_separator);
    let hide_separator = merge_class(style::hide_separator, hide_separator);
    let hover_separator = merge_class(style::hover_separator, hover_separator);

    let is_dragging = XSignal::new("is_dragging", false);
    let drop_separator_class = is_dragging.derive(
        "drop_separator_class",
        move |t| (if *t { &show_separator } else { &hide_separator }).clone(),
        |_, _| None,
    );

    let tab_titles = {
        let li_list = tab_descriptors().map(|tab| {
            let selected = tab.selected(&state);
            li(key = tab.key(), move |li| {
                autoclone!(tab, state, title_class, selected_class, is_dragging);
                tab_title(
                    li,
                    tab.clone(),
                    state.clone(),
                    selected.clone(),
                    title_class.clone(),
                    selected_class.clone(),
                    is_dragging.clone(),
                )
            })
        });
        let li_list = tab_descriptors().cloned().zip(li_list);
        let li_list = li_list.flat_map(|(tab, title)| {
            [
                title,
                li(move |e| {
                    autoclone!(state, drop_separator_class, hover_separator);
                    drop_zone(
                        e,
                        state.clone(),
                        Some(tab.clone()),
                        drop_separator_class.clone(),
                        hover_separator.clone(),
                    )
                }),
            ]
        });
        let li_list = std::iter::once(li(move |e| {
            autoclone!(state, drop_separator_class, hover_separator);
            drop_zone(
                e,
                state.clone(),
                None,
                drop_separator_class.clone(),
                hover_separator.clone(),
            )
        }))
        .chain(li_list);
        div(
            class = titles_class,
            tabs_descriptor
                .before_titles(&state)
                .into_iter()
                .map(Into::into)..,
            ul(li_list..),
            tabs_descriptor
                .after_titles(&state)
                .into_iter()
                .map(Into::into)..,
        )
    };

    let tab_items = {
        let li_list = tab_descriptors().map(|tab| {
            let selected = tab.selected(&state);
            li(key = tab.key(), move |li| {
                autoclone!(tab, state, item_class, selected_class);
                tab_item(
                    li,
                    tab.clone(),
                    state.clone(),
                    selected.clone(),
                    item_class.clone(),
                    selected_class.clone(),
                )
            })
        });
        div(class = items_class, ul(li_list..))
    };

    div(class = tabs_class, [tab_titles, tab_items]..)
}

#[autoclone]
#[template]
#[html]
fn drop_zone<S: TabsState>(
    state: S,
    prev_tab: Option<S::TabDescriptor>,
    #[signal] mut drop_separator_class: XString,
    hover_separator: XString,
) {
    // Hold on to the mutable signal, else the value is frozen.
    let _drop_separator_class = drop_separator_class_mut;

    // TODO: This can be used with `#·57`. Dynamic attributes
    drop(hover_separator);
    li(
        class = drop_separator_class,
        drop = do_or_log(move |ev: web_sys::DragEvent| {
            autoclone!(state);
            ev.prevent_default();
            let dt = ev.data_transfer().ok_or("data_transfer".warn())?;
            let dragged_tab_key = dt.get_data(DRAG_KEY).map_err(|_| "Get DRAG_KEY".warn())?;
            state.move_tab(prev_tab.clone(), dragged_tab_key);
            Ok(())
        }),
        dragover = do_or_log(|ev: web_sys::DragEvent| {
            ev.prevent_default();
            let dt = ev.data_transfer().ok_or("data_transfer".warn())?;
            dt.set_drop_effect("move");
            Ok(())
        }),
        "",
    )
}

#[autoclone]
#[template]
#[html]
fn tab_title<T: TabDescriptor + 'static>(
    tab: T,
    state: T::State,
    #[signal] mut selected: bool,
    title_class: XString,
    selected_class: XString,
    is_dragging: XSignal<bool>,
) {
    let class = if selected {
        format!("{title_class} {selected_class}").into()
    } else {
        title_class
    };
    let key = tab.key();
    return li(
        class = class,
        draggable = true,
        dragstart = do_or_log(move |ev: web_sys::DragEvent| {
            autoclone!(is_dragging);
            let dt = ev.data_transfer().ok_or("data_transfer".warn())?;
            dt.set_data(DRAG_KEY, &key)
                .map_err(|_| "Set DRAG_KEY".warn())?;
            dt.set_effect_allowed("move");
            is_dragging.set(true);
            Ok(())
        }),
        dragend = move |_| is_dragging.set(false),
        click = move |_| selected_mut.set(true),
        [tab.title(&state).into()]..,
    );
}

#[template]
#[html]
fn tab_item<T: TabDescriptor + 'static>(
    tab: T,
    state: T::State,
    #[signal] selected: bool,
    item_class: XString,
    selected_class: XString,
) {
    let class = if selected {
        format!("{item_class} {selected_class}").into()
    } else {
        item_class
    };
    li(class = class, [tab.item(&state).into()]..)
}
