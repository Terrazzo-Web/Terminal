#![allow(unused_crate_dependencies)]

use terrazzo_terminal::RunServerError;

fn main() -> Result<(), RunServerError> {
    terrazzo_terminal::run_server()
}
