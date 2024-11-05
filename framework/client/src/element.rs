use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use named::named;
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

use crate::key::XKey;
use crate::key::KEY_ATTRIBUTE;
use crate::node::XNode;
use crate::prelude::XTemplate;
use crate::signal::depth::Depth;
use crate::signal::reactive_closure::reactive_closure_builder::Consumers;
use crate::string::XString;

mod debug;
mod merge_attributes;
mod merge_children;
mod merge_events;

#[named]
pub struct XElement {
    pub key: XKey,
    pub tag_name: XString,
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

pub struct XAttribute {
    pub name: XString,
    pub value: XString,
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
    pub fn merge(&mut self, depth: Depth, old: &mut Self, element_mut: Rc<Mutex<Element>>) {
        let _span = match &self.key {
            XKey::Named(key) => debug_span!("Merge", %key),
            XKey::Index(_) => trace_span!("Merge", key = ?self.key),
        }
        .entered();
        trace!("Start");
        defer!(trace!("End"));
        self.fix_element_tag(&mut element_mut.lock().unwrap());
        let element: Element = element_mut.lock().expect("element").clone();

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
                    merge_attributes::merge(new_attributes, old_attributes, &element);
                    merge_events::merge(new_events, old_events, &element);
                    merge_children::merge(depth, new_children, old_children, &element);
                }
                XElementValue::Dynamic { .. } | XElementValue::Generated { .. } => {
                    // The reactive callback may still active!
                    old.value = XElementValue::Static {
                        attributes: vec![],
                        events: vec![],
                        children: vec![],
                    };
                    debug!("A node changed from Dynamic/Generated to Static");
                    merge_attributes::merge(new_attributes, &[], &element);
                    merge_events::merge(new_events, &[], &element);
                    merge_children::merge(depth, new_children, &mut [], &element);
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
                    XTemplate::with_depth(depth.next(), element_mut.clone())
                };
                let consumers = new_reactive_callback(new_template.clone());
                self.value = XElementValue::Generated {
                    template: new_template,
                    consumers,
                };
            }
            XElementValue::Generated { .. } => {
                warn!("Illegal XElement state");
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
            tag_name: self.tag_name.clone(),
            value: XElementValue::Static {
                attributes: vec![],
                events: vec![],
                children: vec![],
            },
            before_render: None,
            after_render: None,
        }
    }

    fn fix_element_tag(&self, element: &mut Element) -> Option<()> {
        let old_tag = element.tag_name().to_lowercase();
        let new_tag = self.tag_name.as_str();
        if old_tag.as_str() == new_tag {
            return None;
        }

        info!(old_tag, new_tag, "Changing element tag");
        let document = element.owner_document()?;
        let new_element: Element = document
            .create_element(new_tag)
            .inspect_err(|error| warn!("Create new element '{new_tag}' failed: {error:?}'"))
            .ok()?;
        if let Some(key) = element.get_attribute(KEY_ATTRIBUTE) {
            let () = new_element
                .set_attribute(KEY_ATTRIBUTE, &key)
                .inspect_err(|error| warn!("Set element key failed: {error:?}'"))
                .ok()?;
        }
        let () = element
            .replace_with_with_node_1(&new_element)
            .inspect_err(|error| warn!("Replace element failed: {error:?}'"))
            .ok()?;
        *element = new_element.clone();
        Some(())
    }
}
