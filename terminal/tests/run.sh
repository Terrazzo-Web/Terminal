#!/bin/bash

cd "$(dirname "$0")" || exit
cargo run --locked \
    --bin terrazzo-terminal \
    --no-default-features \
    --features max-level-debug,debug,diagnostics \
    --features server-all \
    -- \
    $@
