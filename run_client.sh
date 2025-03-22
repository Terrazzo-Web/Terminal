#!/bin/bash

cargo run \
    --bin terrazzo-terminal \
    --no-default-features \
    --features server,max_level_info \
    -- \
    --port 3100 \
    --client-name Azerty \
    --gateway-url https://localhost:3000 \
    --gateway-pki /home/richard/.terrazzo/root_ca.cert \
    --auth-code 8552e211-6863-4b35-b8d4-dc6bf19bf94c
