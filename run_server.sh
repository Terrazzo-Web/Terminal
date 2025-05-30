#!/bin/bash

cargo run \
    --bin terrazzo-terminal \
    --no-default-features \
    --features server,max_level_info \
    -- \
    --port 3001 \
    --config-file $PWD/config.toml \
    $@
