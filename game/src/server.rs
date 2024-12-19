#![allow(unused_crate_dependencies)]

#[tokio::main]
async fn main() {
    game::run_server().await
}
