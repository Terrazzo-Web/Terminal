use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

use tracing::error;
use tracing::trace;
use tracing::warn;
use wasm_bindgen::JsCast;
use web_sys::window;
use web_sys::Document;
use web_sys::Element;
use web_sys::Node;
use web_sys::Text;

use super::template::XTemplate;
use super::XElement;
use super::XElementValue;
use crate::key::XKey;
use crate::node::XNode;
use crate::node::XText;
use crate::prelude::OrElseLog as _;

pub fn merge(
    template: &XTemplate,
    new_nodes: &mut [XNode],
    old_nodes: &mut [XNode],
    element: &Element,
) {
    trace! { new_count = new_nodes.len(), old_count = old_nodes.len(), "Children" };
    let document: Document = window().or_throw("window").document().or_throw("document");

    let mut old_elements_map = {
        let mut old_elements_map = HashMap::new();
        for old_node in old_nodes {
            if let XNode::Element(old_element) = old_node {
                old_elements_map.insert(old_element.key.to_owned(), old_element);
            }
        }
        old_elements_map
    };
    let cur_nodes = {
        let mut cur_nodes = vec![];
        let cur_nodes_view = element.child_nodes();
        for index in 0..cur_nodes_view.length() {
            if let Some(cur_node) = cur_nodes_view.item(index) {
                cur_nodes.push(cur_node);
            }
        }
        cur_nodes
    };
    let mut cur_elements = {
        let mut cur_elements = HashMap::new();
        let mut index = 0;
        for cur_node in &cur_nodes {
            if let Some(cur_element) = cur_node.dyn_ref::<Element>() {
                cur_elements.insert(XKey::of(template, index, cur_element), cur_element);
                index += 1;
            }
        }
        cur_elements
    };

    let mut cur_nodes = cur_nodes.iter();

    let mut index = 0;
    for new_node in new_nodes {
        match new_node {
            XNode::Element(new_element) => {
                merge_element(
                    &document,
                    template,
                    element,
                    &mut old_elements_map,
                    &mut cur_nodes,
                    &mut cur_elements,
                    index,
                    new_element,
                );
                index += 1;
            }
            XNode::Text(new_text) => merge_text(&document, element, &mut cur_nodes, new_text),
        }
    }

    detatch_remaining_nodes(element, &mut cur_nodes, None);
}

fn merge_element<'t>(
    document: &Document,
    template: &XTemplate,
    element: &Element,
    old_elements_map: &mut HashMap<XKey, &mut XElement>,
    cur_nodes: &mut impl Iterator<Item = &'t Node>,
    cur_elements: &mut HashMap<XKey, &'t Element>,
    index: usize,
    new_element: &mut XElement,
) {
    let key = &new_element.key;
    trace!("Merge element: index={index:?} key={key:?}",);

    let mut new_element_zero;
    let old_element = match old_elements_map.remove(key) {
        Some(old_element) => {
            trace!("Old element: Found");
            old_element
        }
        None => {
            trace!("Old element: Not found");
            new_element_zero = new_element.zero();
            &mut new_element_zero
        }
    };

    if let Some(cur_element) = get_cur_element(
        template,
        element,
        cur_nodes,
        cur_elements,
        index,
        new_element,
    ) {
        new_element.merge(
            template,
            old_element,
            Rc::new(Mutex::new(cur_element.to_owned())),
        );
        return;
    } else {
        trace!("Cur element: Not found");
    }

    if let Some(cur_element) = create_new_element(document, template, element, new_element) {
        trace!("Cur element: Created new");
        new_element.merge(template, old_element, Rc::new(Mutex::new(cur_element)));
        return;
    }

    trace!("Cur element: Failed");
}

/// Returns `cur_element` that is already attached to the DOM in the right position.
fn get_cur_element<'t>(
    template: &XTemplate,
    element: &Element,
    cur_nodes: &mut impl Iterator<Item = &'t Node>,
    cur_elements: &mut HashMap<XKey, &'t Element>,
    index: usize,
    new_element: &XElement,
) -> Option<&'t Element> {
    let maybe_cur_node = cur_nodes.next();
    let maybe_cur_element = maybe_cur_node.and_then(|cur_node| cur_node.dyn_ref::<Element>());
    if let Some(cur_element) = maybe_cur_element {
        if element_matches(template, index, new_element, cur_element) {
            // This is the most likely path when little has changed: we just need to merge with the next node.
            trace!("Cur element: Found in order");
            return Some(cur_element);
        }
    }

    // Nodes are not in-order, so we detach all nodes.
    detatch_remaining_nodes(element, cur_nodes, maybe_cur_node);

    // Then we either reuse existing nodes from `cur_elements`
    // (or will have to create new nodes.)
    let cur_element = cur_elements.remove(&new_element.key)?;

    // If we found a node, we append it and return it.
    if let Err(error) = element.append_child(cur_element) {
        warn!("Failed to append cur_element: {error:?}");
    }

    trace!("Cur element: Found out of order");
    return Some(cur_element);
}

fn element_matches(
    template: &XTemplate,
    index: usize,
    new_element: &XElement,
    cur_element: &Element,
) -> bool {
    if XKey::of(template, index, cur_element) != new_element.key {
        return false;
    }
    if let XElementValue::Dynamic { .. } = &new_element.value {
        return true;
    }
    let Some(new_tag_name) = new_element.tag_name.as_deref() else {
        return true;
    };
    return cur_element.tag_name().to_lowercase().as_str() == new_tag_name;
}

fn create_new_element(
    document: &Document,
    template: &XTemplate,
    element: &Element,
    new_element: &XElement,
) -> Option<Element> {
    let Some(tag_name) = new_element.tag_name.as_deref() else {
        error!("Failed to create new element with undefined tag name");
        return None;
    };
    let cur_element = document
        .create_element(tag_name)
        .inspect_err(|error| warn!("Create new element '{tag_name}' failed: {error:?}'"))
        .ok()?;

    if let XKey::Named(key) = &new_element.key {
        let () = cur_element
            .set_attribute(template.key_attribute(), key)
            .inspect_err(|error| {
                warn!("Set element key failed: {error:?}'");
            })
            .ok()?;
    }

    if let Err(error) = element.append_child(&cur_element) {
        warn!("Failed to append cur_element: {error:?}");
        return None;
    }

    return Some(cur_element);
}

fn merge_text<'t>(
    document: &Document,
    element: &Element,
    cur_nodes: &mut impl Iterator<Item = &'t Node>,
    XText(new_text): &XText,
) {
    let new_text_data = new_text.as_str();
    trace!("Merge text: {:?}", new_text_data);

    if let Some(cur_text) = get_cur_text(element, cur_nodes) {
        let cur_text_data = cur_text.data();
        if cur_text_data.as_str() != new_text_data {
            cur_text.set_data(new_text_data);
            trace!("Cur text: Update from {cur_text_data:?} to {new_text_data:?}");
        } else {
            trace!("Cur text: Still {:?}", cur_text_data);
        }
        return;
    }

    let cur_text = document.create_text_node(new_text_data);
    if let Err(error) = element.append_child(&cur_text) {
        warn!("Failed to append cur_text: {error:?}");
    }
    trace!("Cur text: Created new");
}

/// Returns `cur_text` that is already attached to the DOM in the right position.
fn get_cur_text<'t>(
    element: &Element,
    cur_nodes: &mut impl Iterator<Item = &'t Node>,
) -> Option<&'t Text> {
    let maybe_cur_node = cur_nodes.next();
    let maybe_cur_text = maybe_cur_node.and_then(|cur_node| cur_node.dyn_ref::<Text>());
    if let Some(cur_text) = maybe_cur_text {
        // This is the most likely path when little has changed: we just need to merge with the next node.
        trace!("Cur text: Found");
        return Some(cur_text);
    }

    trace!("Cur text: Not found");
    detatch_remaining_nodes(element, cur_nodes, maybe_cur_node);
    return None;
}

/// Detaches all the children.
/// `cur_nodes` becomes empty and nodes have to be re-attached from the `cur_elements` HashMap.
fn detatch_remaining_nodes<'t>(
    element: &Element,
    cur_nodes: &mut impl Iterator<Item = &'t Node>,
    maybe_cur_node: Option<&'t Node>,
) {
    if let Some(cur_node) = maybe_cur_node {
        if let Err(error) = element.remove_child(cur_node) {
            warn!("Failed to remove cur_node: {error:?}");
        }
    }
    for cur_node in cur_nodes {
        if let Err(error) = element.remove_child(cur_node) {
            warn!("Failed to remove cur_node: {error:?}");
        }
    }
}
