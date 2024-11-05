use std::collections::HashMap;

use tracing::trace;
use tracing::warn;
use web_sys::Element;

use super::XEvent;

pub fn merge(new_events: &[XEvent], old_events: &[XEvent], element: &Element) {
    trace!(
        new_count = new_events.len(),
        old_count = old_events.len(),
        "Events"
    );

    let mut old_events_map = HashMap::new();
    for old_event in old_events {
        old_events_map.insert(old_event.event_type.to_owned(), &old_event.callback);
    }

    for new_event in new_events {
        let old_event = old_events_map.remove(&new_event.event_type);
        if let Some(old_event_value) = old_event {
            if new_event.callback.as_function() == old_event_value.as_function() {
                trace! { "Event '{}' is still '{:?}'", new_event.event_type, new_event.callback };
                continue;
            }
            if let Err(error) = element.remove_event_listener_with_callback(
                new_event.event_type.as_str(),
                old_event_value.as_function(),
            ) {
                warn! { "Remove old event '{}' failed: {error:?}", new_event.event_type };
            };
        }
        match element.add_event_listener_with_callback(
            new_event.event_type.as_str(),
            new_event.callback.as_function(),
        ) {
            Ok(()) => {
                trace! { "Set event '{}' to '{:?}'", new_event.event_type, new_event.callback };
            }
            Err(error) => {
                warn! { "Set event '{}' to '{:?}' failed: {error:?}", new_event.event_type, new_event.callback };
            }
        }
    }

    for (removed_old_event_name, callback) in old_events_map {
        match element.remove_event_listener_with_callback(
            removed_old_event_name.as_str(),
            callback.as_function(),
        ) {
            Ok(()) => {
                trace! { "Removed event {}", removed_old_event_name };
            }
            Err(error) => {
                warn! { "Removed event {} failed: {error:?}", removed_old_event_name };
            }
        }
    }
}
