# Turn On

[![Translation status](https://translate.codeberg.org/widget/de-swsnr-turnon/de-swsnr-turnon/svg-badge.svg)](https://translate.codeberg.org/engage/de-swsnr-turnon/)

Turn on devices in your network on GNOME:

![The empty start page with the application icon on the left, and the list of devices with discovered devices on the right](./social-image.png)

A small GNOME utility application to send Wake On LAN (WoL) magic packets to devices in a network.

## Installation

- [Flathub](https://flathub.org/apps/de.swsnr.turnon)

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

Copyright Sebastian Wiesner <sebastian@swsnr.de>

Licensed under the EUPL, see <https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12>
