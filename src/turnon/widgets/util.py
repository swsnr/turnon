# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Widget utilities."""

from collections.abc import Iterable

from gi.repository import Gtk


def add_shortcuts(
    widget_class: type[Gtk.Widget], shortcuts: Iterable[tuple[str, str]]
) -> None:
    """Install shortcuts on a widget class.

    `shortcut` is an iterable over `(trigger, action)` tuples, for `Gtk.Shortcut`.
    """
    for trigger, action in shortcuts:
        shortcut = Gtk.Shortcut(
            trigger=Gtk.ShortcutTrigger.parse_string(trigger),
            action=Gtk.ShortcutAction.parse_string(action),
        )
        widget_class.add_shortcut(shortcut)
