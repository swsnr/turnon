#!/usr/bin/bash
set -euo pipefail

toplevel="$(git rev-parse --show-toplevel)"
screenshots="${toplevel}/screenshots"
social_image="${toplevel}/social-image.png"

montage -geometry 602x602+19+19 \
    "${screenshots}"/start-page.png "${screenshots}"/list-of-discovered-devices.png \
    "${social_image}"
oxipng "${social_image}"
