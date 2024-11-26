use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use named::named;
use named::NamedType as _;
use scopeguard::defer;
use tracing::debug;
use tracing::debug_span;
use tracing::info;
use tracing::trace;
use tracing::trace_span;
use tracing::warn;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast as _;
use web_sys::js_sys::Function;
use web_sys::Element;

use self::template::XTemplate;
use crate::attribute::XAttribute;
use crate::key::XKey;
use crate::node::XNode;
use crate::signal::reactive_closure::reactive_closure_builder::Consumers;
use crate::string::XString;
use crate::template::IsTemplate;

mod debug;
mod merge_attributes;
mod merge_children;
mod merge_events;
pub mod template;

#[named]
pub struct XElement {
    pub key: XKey,
    pub tag_name: Option<XString>,
    pub value: XElementValue,
    pub before_render: Option<OnRenderCallback>,
    pub after_render: Option<OnRenderCallback>,
}

pub enum XElementValue {
    Static {
        attributes: Vec<XAttribute>,
        events: Vec<XEvent>,
        children: Vec<XNode>,
    },
    Dynamic(XDynamicElement),
    Generated {
        template: XTemplate,
        consumers: Consumers,
    },
}

pub struct XDynamicElement(Box<dyn Fn(XTemplate) -> Consumers>);

impl<F: Fn(XTemplate) -> Consumers + 'static> From<F> for XDynamicElement {
    fn from(value: F) -> Self {
        Self(Box::new(value))
    }
}

pub struct XEvent {
    pub event_type: XString,
    pub callback: Arc<dyn ClosureAsFunction>,
}

pub trait ClosureAsFunction: std::fmt::Debug {
    fn as_function(&self) -> &Function;
}

impl<T: ?Sized> ClosureAsFunction for Closure<T> {
    fn as_function(&self) -> &Function {
        self.as_ref().unchecked_ref()
    }
}

pub struct OnRenderCallback(pub Box<dyn Fn(Element)>);

impl XElement {
    pub fn merge(&mut self, template: &XTemplate, old: &mut Self, element_rc: Rc<Mutex<Element>>) {
        let _span = match &self.key {
            XKey::Named(key) => debug_span!("Merge", %key),
            XKey::Index(_) => trace_span!("Merge", key = ?self.key),
        }
        .entered();
        trace!("Start");
        defer!(trace!("End"));

        let element = {
            let mut element = element_rc.lock().expect("element");
            if let XKey::Named(new_key) = &self.key {
                let should_update = if let XKey::Named(cur_key) = XKey::of(template, 0, &element) {
                    if new_key != &cur_key {
                        warn!("Templates conflict on key cur_key:{cur_key} vs new_key:{new_key}");
                        true
                    } else {
                        false
                    }
                } else {
                    true
                };
                if should_update {
                    let () = element
                        .set_attribute(template.key_attribute(), new_key)
                        .inspect_err(|error| warn!("Set element key failed: {error:?}'"))
                        .unwrap();
                }
            }

            self.fix_element_tag(template, &mut element);
            element.clone()
        };

        if let Some(OnRenderCallback(before_render)) = &self.before_render {
            before_render(element.clone());
        }

        match &mut self.value {
            XElementValue::Static {
                attributes: new_attributes,
                events: new_events,
                children: new_children,
            } => match &mut old.value {
                XElementValue::Static {
                    attributes: old_attributes,
                    events: old_events,
                    children: old_children,
                } => {
                    merge_attributes::merge(
                        template.depth(),
                        new_attributes,
                        old_attributes,
                        &element,
                    );
                    merge_events::merge(new_events, old_events, &element);
                    merge_children::merge(template, new_children, old_children, &element);
                }
                XElementValue::Dynamic { .. } | XElementValue::Generated { .. } => {
                    // The reactive callback may still active!
                    old.value = XElementValue::Static {
                        attributes: vec![],
                        events: vec![],
                        children: vec![],
                    };
                    debug!("A node changed from Dynamic/Generated to Static");
                    merge_attributes::merge(template.depth(), new_attributes, &mut [], &element);
                    merge_events::merge(new_events, &[], &element);
                    merge_children::merge(template, new_children, &mut [], &element);
                }
            },
            XElementValue::Dynamic(XDynamicElement(new_reactive_callback)) => {
                let new_template = if let XElementValue::Generated {
                    template: old_template,
                    consumers: old_consumers,
                } = &mut old.value
                {
                    trace!("Reuse exising template");
                    let old_consumers = std::mem::take(old_consumers);
                    drop(old_consumers);
                    old_template.clone()
                } else {
                    trace!("Create a new template");
                    XTemplate::with_depth(template.depth().next(), element_rc.clone())
                };
                let consumers = new_reactive_callback(new_template.clone());
                self.value = XElementValue::Generated {
                    template: new_template,
                    consumers,
                };
            }
            XElementValue::Generated { .. } => {
                warn!("Illegal {} state", XElement::type_name());
                debug_assert!(false);
            }
        }

        if let Some(OnRenderCallback(after_render)) = &self.after_render {
            after_render(element);
        }
    }

    pub fn zero(&self) -> XElement {
        XElement {
            key: self.key.to_owned(),
            tag_name: None,
            value: XElementValue::Static {
                attributes: vec![],
                events: vec![],
                children: vec![],
            },
            before_render: None,
            after_render: None,
        }
    }

    fn fix_element_tag(&self, template: &XTemplate, element: &mut Element) -> Option<()> {
        let Some(new_tag) = self.tag_name.as_deref() else {
            return Some(());
        };
        let old_tag = element.tag_name().to_lowercase();
        if old_tag == new_tag {
            return Some(());
        }

        info!(old_tag, new_tag, "Changing element tag");
        let document = element.owner_document()?;
        let new_element: Element = document
            .create_element(new_tag)
            .inspect_err(|error| warn!("Create new element '{new_tag}' failed: {error:?}'"))
            .ok()?;
        if let Some(key) = element.get_attribute(template.key_attribute()) {
            let () = new_element
                .set_attribute(template.key_attribute(), &key)
                .inspect_err(|error| warn!("Set element key failed: {error:?}'"))
                .ok()?;
        }

        // Note: replaceWith() doesn't always work in Chrome when replacing nodes with different tag names.
        let Some(parent) = element.parent_node() else {
            warn!("Node has no parent!");
            return None;
        };
        let insertion = parent.insert_before(&new_element, element.next_sibling().as_ref());
        element.remove();
        insertion
            .inspect_err(|error| warn!("Failed to insert before: {error:?}"))
            .ok()?;
        *element = new_element.clone();
        Some(())
    }
}
