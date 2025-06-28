#[cfg(feature = "method_does_not_exist")]
#[allow(unused)]
fn method_does_not_exist() {
    let x = Some(1);
    let _y = x.unwrap2();
}
