#!/bin/bash

cargo run \
    --bin terrazzo-terminal \
    --no-default-features \
    --features server,max_level_info \
    -- \
    --port 3000 \
    --client-name Gateway \
    --gateway-url https://localhost:3100 \
    --gateway-pki /home/richard/.terrazzo/root_ca.cert \
    --client-certificate /home/richard/.terrazzo/client_certificate_gw \
    --auth-code fc3fd87c-a51d-45f6-a2cb-472d0827ead7
