use std::rc::Rc;

use autoclone_macro::autoclone;
use terrazzo_client::prelude::*;
use terrazzo_macro::html;
use terrazzo_macro::template;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

stylance::import_crate_style!(style, "src/client/widgets/tabs.scss");

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
pub struct TabsOptions<T = Option<XString>> {
    pub tabs_class: T,
    pub titles_class: T,
    pub title_class: T,
    pub items_class: T,
    pub item_class: T,
    pub selected_class: T,
    pub title_show_sep: T,
    pub title_hide_sep: T,
    pub title_drop_zone: T,
    pub title_dropping: T,
    pub title_drop_sep: T,
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
pub fn tabs<T: TabsDescriptor>(
    tabs_descriptor: T,
    state: T::State,
    options: Rc<TabsOptions>,
) -> XElement {
    let options = Rc::new(TabsOptions::base_options().merge(&options));
    let tab_descriptors = || tabs_descriptor.tab_descriptors().iter();
    let is_dragging = XSignal::new("is_dragging", false);

    let drop_zone = move |e, tab| {
        autoclone!(state, is_dragging, options);
        drop_zone(e, state.clone(), tab, is_dragging.clone(), options.clone())
    };

    let tab_titles = {
        let li_list = tab_descriptors().map(|tab| {
            let selected = tab.selected(&state);
            li(key = tab.key(), move |li| {
                autoclone!(tab, state, options, is_dragging);
                tab_title(
                    li,
                    tab.clone(),
                    state.clone(),
                    selected.clone(),
                    options.clone(),
                    is_dragging.clone(),
                )
            })
        });
        let li_list = tab_descriptors().cloned().zip(li_list);
        let li_list = li_list.flat_map(|(tab, title)| {
            [
                title,
                li(move |e| {
                    autoclone!(drop_zone);
                    drop_zone(e, Some(tab.clone()))
                }),
            ]
        });
        let li_list = std::iter::once(li(move |e| {
            autoclone!(drop_zone);
            drop_zone(e, None)
        }))
        .chain(li_list);
        div(
            class = options.titles_class.clone(),
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
                autoclone!(tab, state, options);
                tab_item(
                    li,
                    tab.clone(),
                    state.clone(),
                    selected.clone(),
                    options.clone(),
                )
            })
        });
        div(class = options.items_class.clone(), ul(li_list..))
    };

    div(
        class = options.tabs_class.clone(),
        [tab_titles, tab_items]..,
    )
}

#[autoclone]
#[template]
#[html]
fn drop_zone<S: TabsState>(
    state: S,
    prev_tab: Option<S::TabDescriptor>,
    is_dragging: XSignal<bool>,
    options: Rc<TabsOptions<XString>>,
) -> XElement {
    let drop_zone_active = XSignal::new("drop-zone-active", false);
    li(
        class %= move |a| {
            autoclone!(is_dragging, drop_zone_active, options);
            drop_zone_class(
                a,
                is_dragging.clone(),
                drop_zone_active.clone(),
                options.clone(),
            )
        },
        div(
            class = options.title_drop_zone.clone(),
            style %= move |a: XAttributeTemplate| {
                autoclone!(is_dragging);
                let drop_zone = a.element.clone();
                title_drop_zone_style(a, drop_zone, is_dragging.clone())
            },
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
            dragenter = move |_: web_sys::DragEvent| {
                autoclone!(drop_zone_active);
                drop_zone_active.set(true);
            },
            dragleave = move |_: web_sys::DragEvent| {
                autoclone!(drop_zone_active);
                drop_zone_active.set(false);
            },
        ),
        div(class = options.title_drop_sep.clone()),
    )
}

#[template]
#[html]
fn drop_zone_class(
    #[signal] is_dragging: bool,
    #[signal] drop_zone_active: bool,
    options: Rc<TabsOptions<XString>>,
) -> XAttributeValue {
    let show_or_hide = if is_dragging {
        &options.title_show_sep
    } else {
        &options.title_hide_sep
    };
    if drop_zone_active {
        format!("{show_or_hide} {}", options.title_dropping).into()
    } else {
        show_or_hide.clone().into()
    }
}

#[template]
#[html]
fn title_drop_zone_style(drop_zone: Element, #[signal] is_dragging: bool) -> XAttributeValue {
    if !is_dragging {
        return "".into();
    }
    let drop_zone: &HtmlElement = drop_zone.dyn_ref().unwrap();
    let li_sep = drop_zone.parent_element().unwrap();
    let li_sep: &HtmlElement = li_sep.dyn_ref().unwrap();
    let offset_left = li_sep.offset_left();
    format!("left: calc({offset_left}px - var(--sep-zone)/2);").into()
}

#[autoclone]
#[template]
#[html]
fn tab_title<T: TabDescriptor + 'static>(
    tab: T,
    state: T::State,
    #[signal] mut selected: bool,
    options: Rc<TabsOptions<XString>>,
    is_dragging: XSignal<bool>,
) -> XElement {
    let class = if selected {
        format!("{} {}", options.title_class, options.selected_class).into()
    } else {
        options.title_class.clone()
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
        tab.title(&state).into(),
    );
}

#[template]
#[html]
fn tab_item<T: TabDescriptor + 'static>(
    tab: T,
    state: T::State,
    #[signal] selected: bool,
    options: Rc<TabsOptions<XString>>,
) -> XElement {
    let class = if selected {
        format!("{} {}", options.item_class, options.selected_class).into()
    } else {
        options.item_class.clone()
    };
    li(class = class, [tab.item(&state).into()]..)
}

mod tab_options {
    use terrazzo_client::prelude::XString;

    use super::style;
    use super::TabsOptions;

    impl TabsOptions<&'static str> {
        pub const fn base_options() -> Self {
            Self {
                tabs_class: style::tabs,
                titles_class: style::titles,
                title_class: style::title,
                items_class: style::items,
                item_class: style::item,
                selected_class: style::selected,
                title_show_sep: style::title_show_sep,
                title_hide_sep: &style::title_hide_sep,
                title_drop_zone: style::title_drop_zone,
                title_dropping: style::title_dropping,
                title_drop_sep: style::title_drop_sep,
            }
        }

        pub fn merge<O: std::fmt::Display>(
            &self,
            new: &TabsOptions<Option<O>>,
        ) -> TabsOptions<XString> {
            TabsOptions::<XString> {
                tabs_class: Self::merge_class(&self.tabs_class, &new.tabs_class),
                titles_class: Self::merge_class(&self.titles_class, &new.titles_class),
                title_class: Self::merge_class(&self.title_class, &new.title_class),
                items_class: Self::merge_class(&self.items_class, &new.items_class),
                item_class: Self::merge_class(&self.item_class, &new.item_class),
                selected_class: Self::merge_class(&self.selected_class, &new.selected_class),
                title_show_sep: Self::merge_class(&self.title_show_sep, &new.title_show_sep),
                title_hide_sep: Self::merge_class(&self.title_hide_sep, &new.title_hide_sep),
                title_drop_zone: Self::merge_class(&self.title_drop_zone, &new.title_drop_zone),
                title_dropping: Self::merge_class(&self.title_dropping, &new.title_dropping),
                title_drop_sep: Self::merge_class(&self.title_drop_sep, &new.title_drop_sep),
            }
        }

        fn merge_class(
            base_class: &'static str,
            override_class: &Option<impl std::fmt::Display>,
        ) -> XString {
            override_class
                .as_ref()
                .map(|override_class| format!("{} {}", base_class, override_class).into())
                .unwrap_or(base_class.into())
        }
    }
}
