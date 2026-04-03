# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""The main application of Turn On."""

import os
import sys
from gettext import gettext as _
from gettext import pgettext as C_
from pathlib import Path
from typing import override

from gi.repository import Adw, Gio, GLib, Gtk

import turnon
from turnon.cli import AppCLI

from . import log
from .model import Device, DeviceStorage
from .model.storage import load_devices
from .widgets import EditDeviceDialog, TurnOnApplicationWindow


class TurnOnApplication(Adw.Application):
    """The main application."""

    __gtype_name__: str = "TurnOnApplication"

    def __init__(self, application_id: str) -> None:
        """Create a new application with the given ID."""
        super().__init__(
            application_id=application_id,
            resource_base_path="/de/swsnr/turnon",
            flags=Gio.ApplicationFlags.HANDLES_COMMAND_LINE,
        )
        self._settings: Gio.Settings = Gio.Settings.new(application_id)
        self._registered_devices = Gio.ListStore[Device].new(Device)
        self._devices_file: Path = (
            Path(GLib.get_user_data_dir()) / application_id / "devices.json"
        )
        self._add_options()
        self._setup_actions()
        self._device_storage: DeviceStorage | None = None

    def _add_options(self) -> None:
        self.add_main_option(
            "list-devices",
            0,
            GLib.OptionFlags.NONE,
            GLib.OptionArg.NONE,
            C_("option.list-devices.description", "List all devices and their status"),
        )
        self.add_main_option(
            "add-device",
            0,
            GLib.OptionFlags.NONE,
            GLib.OptionArg.NONE,
            C_("option.add-device.description", "Add a new device"),
        )
        self.add_main_option(
            "devices-file",
            0,
            GLib.OptionFlags.HIDDEN,
            GLib.OptionArg.FILENAME,
            C_(
                "option.devices-file.description",
                "Use the given file as storage for devices (for development only)",
            ),
        )
        self.add_main_option(
            "main-window-height",
            0,
            GLib.OptionFlags.HIDDEN,
            GLib.OptionArg.INT,
            C_(
                "option.main-window-height.description",
                "Set the height of the main window (for development only)",
            ),
            C_(
                "option.main-window-height.arg.description",
                "HEIGHT",
            ),
        )

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

    def _new_device_saved(self, _dialog: EditDeviceDialog, device: Device) -> None:
        self._registered_devices.append(device)

    def _activate_add(
        self, _act: Gio.SimpleAction, _parameter: GLib.Variant | None = None
    ) -> None:
        dialog = EditDeviceDialog()
        dialog.connect("saved", self._new_device_saved)
        dialog.present(self.get_active_window())

    def _setup_actions(self) -> None:
        quit = Gio.SimpleAction(name="quit")
        _ = quit.connect("activate", lambda *args: self.quit())
        about = Gio.SimpleAction(name="about")
        _ = about.connect("activate", self._activate_about)
        add = Gio.SimpleAction(name="add-device")
        add.connect("activate", self._activate_add)
        for action in [about, quit, add]:
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
    def do_handle_local_options(self, options: GLib.VariantDict) -> int:
        _ = Adw.Application.do_handle_local_options(self, options)

        path = options.lookup_value("devices-file")
        if path:
            self._devices_file = Path(
                path.get_bytestring().decode(sys.getfilesystemencoding())
            )
            log.warn(
                f"Overriding storage file to {self._devices_file}; "
                + "only use for development purposes!",
            )

        # Apparently, -1 makes command line handling continue
        return -1

    @override
    def do_command_line(self, command_line: Gio.ApplicationCommandLine) -> int:
        _ = Adw.Application.do_command_line(self, command_line)
        options = command_line.get_options_dict()

        list_devices = options.lookup_value("list-devices")
        if list_devices and list_devices.unpack():
            return AppCLI(self, self._registered_devices, command_line).list_devices()

        self.activate()
        height = options.lookup_value("main-window-height")
        if height:
            height = height.get_int32()
            log.warn(f"Overriding main window height {height} from command line")
            window = self.get_active_window()
            if window:
                window.props.height_request = height
        add_device = options.lookup_value("add-device")
        if add_device and add_device.unpack():
            self.activate_action("add-device")
        return os.EX_OK

    @override
    def do_startup(self) -> None:
        Adw.Application.do_startup(self)

        app_id = self.get_application_id()
        assert app_id is not None
        Gtk.Window.set_default_icon_name(app_id)

        self._registered_devices.remove_all()
        for device in load_devices(self._devices_file):
            self._registered_devices.append(Device(device))

        # Automatically save devices
        self._device_storage = DeviceStorage(self._devices_file)
        self._device_storage.start()
        self._device_storage.save_automatically(self._registered_devices)

    @override
    def do_activate(self) -> None:
        Adw.Application.do_activate(self)

        app_id = self.get_application_id()
        assert app_id is not None

        window = self.get_active_window()
        if not window:
            window = TurnOnApplicationWindow(self, self._registered_devices)
            if app_id.endswith(".Devel"):
                window.add_css_class("devel")

            flags = Gio.SettingsBindFlags.DEFAULT
            self._settings.bind("main-window-width", window, "default-width", flags)
            self._settings.bind("main-window-height", window, "default-height", flags)
            self._settings.bind("main-window-maximized", window, "maximized", flags)
            self._settings.bind("main-window-fullscreen", window, "fullscreened", flags)

        window.present()

    @override
    def do_shutdown(self) -> None:
        Adw.Application.do_shutdown(self)
        if self._device_storage:
            self._device_storage.request_stop()
            self._device_storage.join()
            self._device_storage = None
