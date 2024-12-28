use std::fmt::Debug;

use tracing::debug;
use tracing::debug_span;
use tracing::trace;
use tracing::warn;

use super::depth::Depth;
use super::XSignal;
use crate::debug_correlation_id::DebugCorrelationId;
use crate::string::XString;

impl<T> XSignal<T> {
    pub fn derive<U>(
        &self,
        name: impl Into<XString>,
        to: impl Fn(&T) -> U + 'static,
        from: impl Fn(&T, &U) -> Option<T> + 'static,
    ) -> XSignal<U>
    where
        T: Debug + 'static,
        U: Debug + Eq + 'static,
    {
        let main = self;
        let main_weak = main.downgrade();
        let main_name =
            DebugCorrelationId::new(|| format!("Main:{}", main.0.producer.name()).into());

        let derived = XSignal::new(name, to(main.0.current_value.lock().unwrap().value()));
        let derived_weak = derived.downgrade();
        let derived_name =
            DebugCorrelationId::new(|| format!("Derived:{}", derived.0.producer.name()).into());

        let span = debug_span! { "Derived signal", main = %main_name, derived = %derived_name };
        let _span = span.clone().entered();
        debug!(main = %main_name, derived = %derived_name, "Make");

        {
            // Update main when derived changes
            let main_weak = main_weak.clone();
            let derived_weak = derived_weak.clone();
            let span = span.clone();
            trace!("Derived updates Main");
            let derived_producer = &derived.0.producer;
            let consumer =
                derived_producer.register(derived_name, Depth::zero(), move |_version| {
                    let _span = span.enter();
                    debug!("Derived updated");
                    let Some(derived) = derived_weak.upgrade() else {
                        warn!("Derived is dropped but triggered");
                        debug_assert!(false);
                        return;
                    };
                    let Some(main) = main_weak.upgrade() else {
                        warn!("Main is dropped but subscribed");
                        debug_assert!(false);
                        return;
                    };
                    let t = from(
                        main.0.current_value.lock().unwrap().value(),
                        derived.0.current_value.lock().unwrap().value(),
                    );
                    if let Some(t) = t {
                        main.force(t);
                    }
                });
            // If main is dropped there is no need to try to update it.
            let mut on_main_drop = main.0.on_drop.lock().unwrap();
            on_main_drop.push(Box::new(move || drop(consumer)));
        }

        {
            // Update derived when main changes
            let main_weak = main_weak.clone();
            let derived_weak = derived_weak.clone();
            let span = span.clone();
            trace!("Main updates Derived");
            let main_producer = &main.0.producer;
            let consumer = main_producer.register(main_name, Depth::zero(), move |_version| {
                let _span = span.enter();
                debug!("Main updated");
                let Some(main) = main_weak.upgrade() else {
                    warn!("Main is dropped but triggered");
                    debug_assert!(false);
                    return;
                };
                let Some(derived) = derived_weak.upgrade() else {
                    warn!("Derived is dropped but subscribed");
                    debug_assert!(false);
                    return;
                };
                let t = to(main.0.current_value.lock().unwrap().value());
                derived.set(t);
            });
            // If derived is dropped there is no need to try to update it.
            let mut on_drop = derived.0.on_drop.lock().unwrap();
            on_drop.push(Box::new(move || drop(consumer)));
        }

        return derived;
    }

    pub fn view<U>(&self, name: impl Into<XString>, to: impl Fn(&T) -> U + 'static) -> XSignal<U>
    where
        T: Debug + 'static,
        U: Debug + Eq + 'static,
    {
        self.derive(name, to, |_, _| None)
    }
}

pub fn if_change<T: Eq, U>(
    from: impl Fn(&T, &U) -> Option<T> + 'static,
) -> impl Fn(&T, &U) -> Option<T> + 'static {
    move |old_t, u| from(old_t, u).filter(|new_t| new_t != old_t)
}
