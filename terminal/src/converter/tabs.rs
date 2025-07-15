#![cfg(feature = "client")]

use terrazzo::html;
use terrazzo::prelude::*;
use terrazzo::widgets::tabs::TabDescriptor;
use terrazzo::widgets::tabs::TabsDescriptor;
use terrazzo::widgets::tabs::TabsState;

use super::api::Conversion;
use super::api::Conversions;

impl TabsDescriptor for Conversions {
    type State = ConversionsState;
    type TabDescriptor = Conversion;

    fn tab_descriptors(&self) -> &[Self::TabDescriptor] {
        &self.conversions
    }
}

#[derive(Clone)]
pub struct ConversionsState {
    selected: XSignal<Conversion>,
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
        let language = &self.language.name;
        div("{language}")
    }

    #[html]
    fn item(&self, _state: &Self::State) -> impl Into<XNode> {
        let content = &self.content;
        pre("{content}")
    }

    fn selected(&self, state: &Self::State) -> XSignal<bool> {
        let language = self.language.clone();
        let this = self.clone();
        state.selected.derive(
            format!("selected-{language}"),
            move |conversion| conversion.language == language,
            move |_, selected| selected.then(|| this.clone()),
        )
    }
}
