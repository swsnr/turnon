# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""The main application of Turn On."""

from typing import override

from gi.repository import Adw, Gio, Gtk

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

    def _setup_actions(self) -> None:
        pass

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
