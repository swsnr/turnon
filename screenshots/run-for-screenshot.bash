#!/usr/bin/env bash

set -euo pipefail

DEVICES_FILE="${1:-"$(git rev-parse --show-toplevel)/screenshots/devices.json"}"

variables=(
    # Run app with default settings: Force the in-memory gsettings backend to
    # block access to standard Gtk settings, and tell Adwaita not to access
    # portals to prevent it from getting dark mode and accent color from desktop
    # settings.
    #
    # Effectively this makes our app run with default settings.
    GSETTINGS_BACKEND=memory
    ADW_DISABLE_PORTAL=1
)

exec env "${variables[@]}" cargo run -- \
    --devices-file "${DEVICES_FILE}" \
    --arp-cache-file "$(git rev-parse --show-toplevel)/screenshots/arp"
