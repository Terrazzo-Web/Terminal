#![cfg(feature = "client")]

use std::collections::HashMap;
use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::widgets::tabs::TabDescriptor;
use terrazzo::widgets::tabs::TabsDescriptor;
use terrazzo::widgets::tabs::TabsState;

use super::api::Conversion;
use super::api::Conversions;
use crate::converter::api::Language;

impl TabsDescriptor for Conversions {
    type State = ConversionsState;
    type TabDescriptor = Conversion;

    fn tab_descriptors(&self) -> &[Self::TabDescriptor] {
        &self.conversions
    }
}

#[derive(Clone)]
pub struct ConversionsState {
    #[expect(unused)]
    selected: XSignal<Option<Conversion>>,
    selected_tabs: Arc<HashMap<Language, XSignal<bool>>>,
}

impl ConversionsState {
    #[autoclone]
    pub fn new(conversions: &Conversions, preferred_language: XSignal<Option<Language>>) -> Self {
        let current_preferred_language = preferred_language.get_value_untracked();
        let selected = XSignal::new(
            "conversion-selected",
            current_preferred_language
                .map(|current_preferred_language| {
                    conversions
                        .conversions
                        .iter()
                        .find(|conversion| conversion.language == current_preferred_language)
                        .cloned()
                })
                .flatten(),
        );
        let selected_tabs = conversions
            .conversions
            .iter()
            .map(|conversion| {
                let this = conversion.clone();
                let language = this.language.clone();
                let is_selected = selected.derive(
                    format!("selected-{language}"),
                    move |conversion| {
                        autoclone!(language);
                        conversion
                            .as_ref()
                            .map(|c: &Conversion| c.language == language)
                            .unwrap_or(false)
                    },
                    move |_, selected| {
                        autoclone!(preferred_language);
                        selected.then(|| {
                            preferred_language.set(this.language.clone());
                            Some(this.clone())
                        })
                    },
                );
                (language, is_selected)
            })
            .collect::<HashMap<_, _>>()
            .into();
        Self {
            selected,
            selected_tabs,
        }
    }
}

impl TabsState for ConversionsState {
    type TabDescriptor = Conversion;
    fn move_tab(&self, _after_tab: Option<Self::TabDescriptor>, _moved_tab_key: String) {}
}

impl TabDescriptor for Conversion {
    type State = ConversionsState;

    fn key(&self) -> XString {
        self.language.name.clone().into()
    }

    #[html]
    fn title(&self, _state: &Self::State) -> impl Into<XNode> {
        let language = self.language.name.clone();
        terrazzo::widgets::link::link(
            |_click| {},
            move || [span(class = super::ui::style::title_span, "{language}")],
        )
    }

    #[html]
    fn item(&self, _state: &Self::State) -> impl Into<XNode> {
        let content = &self.content;
        pre("{content}")
    }

    fn selected(&self, state: &Self::State) -> XSignal<bool> {
        state.selected_tabs.get(&self.language).unwrap().clone()
    }
}
