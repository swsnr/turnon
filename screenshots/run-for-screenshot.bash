#!/usr/bin/env bash
exec env G_MESSAGES_DEBUG=all GSETTINGS_BACKEND=memory cargo run -- \
    --devices-file "$(git rev-parse --show-toplevel)/screenshots/devices.json"
