#!/bin/bash

cargo run \
    --bin terrazzo-terminal \
    --no-default-features \
    --features server,max_level_info \
    -- \
    --port 3101 \
    --client-name Qwerty \
    --gateway-url https://localhost:3100 \
    --gateway-pki /home/richard/.terrazzo/root_ca.cert \
    --auth-code 282f3d60-3771-421a-8bb4-7aa225df3aed \
    --client-certificate /home/richard/.terrazzo/client_certificate2
