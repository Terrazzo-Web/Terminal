#!/bin/bash

cargo run \
    --bin terrazzo-terminal \
    --no-default-features \
    --features server,max_level_debug \
    -- \
    --port 3001 \
    --config-file $PWD/config.toml \
    $@
