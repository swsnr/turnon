# Turn On

[![Latest release](https://img.shields.io/github/v/release/swsnr/turnon)](https://github.com/swsnr/turnon/releases/)
[![Flathub Version](https://img.shields.io/flathub/v/de.swsnr.turnon)](https://flathub.org/apps/de.swsnr.turnon)
[![Translation status](https://translate.codeberg.org/widget/de-swsnr-turnon/de-swsnr-turnon/svg-badge.svg)](https://translate.codeberg.org/engage/de-swsnr-turnon/)
[![CI status](https://img.shields.io/github/actions/workflow/status/swsnr/turnon/test.yml)](https://github.com/swsnr/turnon/actions)
[![Package build result](https://build.opensuse.org/projects/home:swsnr:turnon/packages/turnon/badge.svg?type=default)](https://build.opensuse.org/package/show/home:swsnr:turnon/turnon)

Turn on devices in your network on GNOME:

![The empty greeting page with the application icon and a button to add a new device](./screenshots/start-page.png)

![Two devices, one of them on, and the other off](./screenshots/list-of-devices.png)

A small GNOME utility application to send Wake On LAN (WoL) magic packets to devices in a network.

## Installation

- [Flathub](https://flathub.org/apps/de.swsnr.turnon)
- [Arch binary package](https://build.opensuse.org/project/show/home:swsnr:turnon).

## Translations

Please submit translations to <https://translate.codeberg.org/engage/de-swsnr-turnon/>.

## Troubleshooting

Obtain a debugging log with:

```console
$ flatpak run --env=G_MESSAGES_DEBUG=all de.swsnr.turnon
```

You may want to add this to bug reports.

## License

This Program is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at <http://mozilla.org/MPL/2.0/>.
