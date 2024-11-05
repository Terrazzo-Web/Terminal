use std::collections::HashMap;

use tracing::trace;
use tracing::warn;
use web_sys::Element;

use super::XAttribute;

pub fn merge(new_attributes: &[XAttribute], old_attributes: &[XAttribute], element: &Element) {
    trace!(
        new_count = new_attributes.len(),
        old_count = old_attributes.len(),
        "Attributes"
    );

    let mut old_attributes_map = HashMap::new();
    for old_attribute in old_attributes {
        old_attributes_map.insert(old_attribute.name.to_owned(), &old_attribute.value);
    }

    for new_attribute in new_attributes {
        let old_attribute = old_attributes_map.remove(&new_attribute.name);
        if let Some(old_attribute_value) = old_attribute {
            if new_attribute.value == *old_attribute_value {
                trace! { "Attribute '{}' is still '{}'", new_attribute.name, new_attribute.value };
                continue;
            }
        }
        match element.set_attribute(new_attribute.name.as_str(), new_attribute.value.as_str()) {
            Ok(()) => {
                trace! { "Set attribute '{}' to '{}'", new_attribute.name, new_attribute.value };
            }
            Err(error) => {
                warn! { "Set attribute '{}' to '{}' failed: {error:?}", new_attribute.name, new_attribute.value };
            }
        }
    }

    for removed_old_attribute_name in old_attributes_map.keys() {
        match element.remove_attribute(removed_old_attribute_name.as_str()) {
            Ok(()) => {
                trace! { "Removed attribute {}", removed_old_attribute_name };
            }
            Err(error) => {
                warn! { "Removed attribute {} failed: {error:?}", removed_old_attribute_name };
            }
        }
    }
}
