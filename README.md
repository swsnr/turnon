# Turn On

[![Latest release](https://img.shields.io/github/v/release/swsnr/turnon)](https://github.com/swsnr/turnon/releases/)
[![Translation status](https://translate.codeberg.org/widget/de-swsnr-turnon/de-swsnr-turnon/svg-badge.svg)](https://translate.codeberg.org/engage/de-swsnr-turnon/)
[![CI status](https://img.shields.io/github/actions/workflow/status/swsnr/turnon/test.yml)](https://github.com/swsnr/turnon/actions)

Turn on devices in your network on GNOME:

![The empty start page with the application icon on the left, and the list of devices with discovered devices on the right](./social-image.png)

A small GNOME utility application to send Wake On LAN (WoL) magic packets to devices in a network.

## Installation

- [Flathub](https://flathub.org/apps/de.swsnr.turnon)
- [Arch binary package](https://build.opensuse.org/project/show/home:swsnr:turnon).

## Translations

Please submit translations to <https://translate.codeberg.org/engage/de-swsnr-turnon/>.

## Troubleshooting

Turn On provides a troubleshooting report under "Menu" -> "About Turn On" -> "Troubleshooting" -> "Debugging information".
Additionally, you can obtain a debugging log by running Turn On as follows from a terminal:

```console
$ flatpak run --env=G_MESSAGES_DEBUG=all de.swsnr.turnon
```

You may want to add both to your bug reports, but note that both may contain **sensitive information**.
Read them carefully before sharing them publicly; in doubt, ask to share them privately in your bug report.

## License

This Program is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at <http://mozilla.org/MPL/2.0/>.
