#!/bin/bash

cd "$(dirname "$0")" || exit
cargo run \
    --bin terrazzo-terminal \
    --no-default-features \
    --features server,max_level_info \
    -- \
    $@
