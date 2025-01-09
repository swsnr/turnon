#!/usr/bin/bash
set -euo pipefail
montage -geometry 602x602+19+19 start-page.png list-of-discovered-devices.png social-image.png
oxipng social-image.png
