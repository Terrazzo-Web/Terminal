#!/bin/bash

cd "$(dirname "$0")" || exit
cargo run --locked \
    --bin terrazzo-terminal \
    --no-default-features \
    --features debug,diagnostics \
    --features max-level-debug \
    --features server-all \
    -- \
    $@
