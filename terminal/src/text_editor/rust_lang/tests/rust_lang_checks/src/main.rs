fn main() {
    println!("Hello, world!");
}

#[cfg(feature = "some_unused_method")]
fn some_unused_method() {}
