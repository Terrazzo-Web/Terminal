#![cfg(test)]

use super::XSignal;
use crate::signal::derive::if_change;

#[test]
fn derive() {
    setup_logs();
    let main = XSignal::new("main", 1i32);
    let derived = main.derive(
        "derived",
        |main: &i32| (main + 1) as usize,
        if_change(|_main: &i32, derived: &usize| Some((derived - 1) as i32)),
    );

    assert_eq!(&1, main.0.current_value.lock().unwrap().value());
    assert_eq!(&2, derived.0.current_value.lock().unwrap().value());

    derived.set(3usize);
    assert_eq!(&2, main.0.current_value.lock().unwrap().value());
    assert_eq!(&3, derived.0.current_value.lock().unwrap().value());

    main.set(5i32);
    assert_eq!(&5, main.0.current_value.lock().unwrap().value());
    assert_eq!(&6, derived.0.current_value.lock().unwrap().value());
}

#[test]
fn drop_main() {
    setup_logs();
    let main = XSignal::new("main", 1i32);
    let derived = main.derive(
        "derived",
        |main: &i32| (main + 1) as usize,
        if_change(|_main: &i32, derived: &usize| Some((derived - 1) as i32)),
    );

    assert_eq!(1, derived.0.producer.consumers().count());
    drop(main);
    assert_eq!(0, derived.0.producer.consumers().count());
    derived.set(3usize);
    assert_eq!(0, derived.0.producer.consumers().count());
    assert_eq!(&3, derived.0.current_value.lock().unwrap().value());
}

#[test]
fn drop_derived() {
    setup_logs();
    let main = XSignal::new("main", 1i32);
    let derived = main.derive(
        "derived",
        |main: &i32| (main + 1) as usize,
        if_change(|_main: &i32, derived: &usize| Some((derived - 1) as i32)),
    );

    assert_eq!(1, main.0.producer.consumers().count());
    drop(derived);
    assert_eq!(0, main.0.producer.consumers().count());
    main.set(3i32);
    assert_eq!(0, main.0.producer.consumers().count());
    assert_eq!(&3, main.0.current_value.lock().unwrap().value());
}

fn setup_logs() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .try_init();
}
