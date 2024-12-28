use std::rc::Rc;

use autoclone_macro::autoclone;
use terrazzo_client::prelude::*;
use terrazzo_macro::html;
use terrazzo_macro::template;

stylance::import_crate_style!(style, "src/widgets/tabs.scss");

pub trait TabsDescriptor: Clone + 'static {
    type State: Clone;
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
pub fn tabs<T: TabsDescriptor + 'static>(
    tabs_descriptor: T,
    state: T::State,
    tabs_options: Rc<TabsOptions>,
) {
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
    } = &*tabs_options;

    let tab_descriptors = || tabs_descriptor.tab_descriptors().iter();
    let tabs_class = merge_class(style::tabs, tabs_class);
    let titles_class = merge_class(style::titles, titles_class);
    let title_class = merge_class(style::title, title_class);
    let items_class = merge_class(style::items, items_class);
    let item_class = merge_class(style::item, item_class);
    let selected_class = merge_class(style::selected, selected_class);

    let tab_titles = {
        let li_list = tab_descriptors().map(|tab| {
            let selected = tab.selected(&state);
            li(key = tab.key(), move |li| {
                autoclone!(tab, state, title_class, selected_class);
                tab_title(
                    li,
                    tab.clone(),
                    state.clone(),
                    selected.clone(),
                    title_class.clone(),
                    selected_class.clone(),
                )
            })
        });
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

#[template]
#[html]
fn tab_title<T: TabDescriptor + 'static>(
    tab: T,
    state: T::State,
    #[signal] mut selected: bool,
    title_class: XString,
    selected_class: XString,
) {
    let class = if selected {
        format!("{title_class} {selected_class}").into()
    } else {
        title_class
    };
    return li(
        class = class,
        click = move |_ev| selected_mut.set(true),
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
