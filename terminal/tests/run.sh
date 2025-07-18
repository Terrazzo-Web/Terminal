#!/bin/bash

cd "$(dirname "$0")" || exit
cargo run \
    --bin terrazzo-terminal \
    --no-default-features \
    --features server,max-level-debug,debug,diagnostics \
    --features converter-server,terminal-server \
    -- \
    $@
