# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""The main application of Turn On."""

from gettext import gettext as _
from gettext import pgettext as C_
from typing import override

from gi.repository import Adw, Gio, GLib, Gtk

import turnon

from .widgets import TurnOnApplicationWindow


class TurnOnApplication(Adw.Application):
    """The main application."""

    __gtype_name__: str = "TurnOnApplication"

    def __init__(self, application_id: str) -> None:
        """Create a new application with the given ID."""
        super().__init__(
            application_id=application_id, resource_base_path="/de/swsnr/turnon"
        )
        self._settings: Gio.Settings = Gio.Settings.new(application_id)

    def _activate_about(
        self, _act: Gio.SimpleAction, _parameter: GLib.Variant | None = None
    ) -> None:
        version = self.get_version()
        assert version is not None
        (major, minor) = version.split(".")[:2]
        dialog = Adw.AboutDialog.new_from_appdata(
            "/de/swsnr/turnon/metainfo.xml", f"{major}.{minor}.0"
        )
        dialog.set_version(version)
        dialog.set_license_type(Gtk.License.CUSTOM)
        dialog.set_license(
            C_(
                "about-dialog.license-text",
                # Translators: This is Pango markup, be sure to escape appropriately
                """Copyright {copyright_name} &lt;{copyright_email}&gt;

Licensed under the terms of the EUPL 1.2. You can find official translations
of the license text at <a href=\"{translations}\">{translations}</a>.

The full English text follows.

{license_text}""",
            ).format(
                copyright_name="Sebastian Wiesner",
                copyright_email="sebastian@swsnr.de",
                translations="https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12",
                license_text=GLib.markup_escape_text(turnon.license_text()),
            )
        )
        dialog.add_link(
            C_("about-dialog.link.label", "Translations"),
            "https://translate.codeberg.org/engage/de-swsnr-turnon/",
        )

        dialog.set_developers(["Sebastian Wiesner https://swsnr.de"])
        dialog.set_designers(["Sebastian Wiesner https://swsnr.de"])
        # Credits for the translator to the current language.
        # Translators: Add your name here, as "Jane Doe <jdoe@example.com>" or
        # "Jane Doe https://jdoe.example.com". Mail address or URL are optional.
        # Separate multiple translators with a newline, i.e. \n
        dialog.set_translator_credits(_("translator-credits"))
        dialog.add_acknowledgement_section(
            C_(
                "about-dialog.acknowledgment-section",
                "Helpful services",
            ),
            [
                "Codeberg https://codeberg.org",
                "Flathub https://flathub.org/",
                "Open Build Service https://build.opensuse.org/",
            ],
        )
        dialog.add_other_app(
            "de.swsnr.pictureoftheday",
            # Translators: Use app name from https://flathub.org/apps/de.swsnr.pictureoftheday
            C_("about-dialog.other-app.name", "Picture Of The Day"),
            C_(
                "about-dialog.other-app.summary",
                # Translators: Use summary from https://flathub.org/apps/de.swsnr.pictureoftheday
                "Your daily wallpaper",
            ),
        )
        dialog.present(self.get_active_window())

    def _setup_actions(self) -> None:
        quit = Gio.SimpleAction(name="quit")
        _ = quit.connect("activate", lambda *args: self.quit())
        about = Gio.SimpleAction(name="about")
        _ = about.connect("activate", self._activate_about)
        for action in [about, quit]:
            self.add_action(action)

        # We do _not_ add a global shortcut for app.add-device because we handle adding
        # devices a bit more flexible:
        #
        # Device rows have their own action to add a new device, which enables adding
        # a new device from a discovered device with pre-filled fields.  We use the
        # Ctrl+N shortcut for this action too, so that the user gets a prefilled dialog
        # when they press Ctrl+N while a discovered device is focused, or an empty
        # dialog otherwise.
        #
        # If we set a global shortcut here, it'd always override the shortcut of the
        # device rows, and users would always just get the empty dialog.
        self.set_accels_for_action("window.close", ["<Control>w"])
        self.set_accels_for_action("app.quit", ["<Control>q"])

    @override
    def do_startup(self) -> None:
        Adw.Application.do_startup(self)

        app_id = self.get_application_id()
        assert app_id is not None
        Gtk.Window.set_default_icon_name(app_id)

        self._setup_actions()

        # TODO: load devices

    @override
    def do_activate(self) -> None:
        Adw.Application.do_activate(self)

        app_id = self.get_application_id()
        assert app_id is not None

        window = self.get_active_window()
        if not window:
            window = TurnOnApplicationWindow(self)
            if app_id.endswith(".Devel"):
                window.add_css_class("devel")

            # TODO: Bind devices model

            flags = Gio.SettingsBindFlags.DEFAULT
            self._settings.bind("main-window-width", window, "default-width", flags)
            self._settings.bind("main-window-height", window, "default-height", flags)
            self._settings.bind("main-window-maximized", window, "maximized", flags)
            self._settings.bind("main-window-fullscreen", window, "fullscreened", flags)

        window.present()
