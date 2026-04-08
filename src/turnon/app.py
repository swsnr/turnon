# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""The main application of Turn On."""

import asyncio
import os
import sys
from functools import partial
from gettext import gettext as _
from gettext import pgettext as C_
from ipaddress import IPv4Address, IPv4Interface
from itertools import islice
from pathlib import Path
from typing import override

from gi.repository import Adw, Gio, GLib, GObject, Gtk

import turnon

from . import log
from .cli import AppCLI
from .dbus import DBusObject
from .model import Device, DeviceObject, DeviceStorage
from .model.storage import load_devices
from .net import SocketAddress
from .net.arp import ArpCacheEntry, ArpFlag, ArpHardwareType
from .searchprovider import SearchProvider, search_provider_interface
from .util import gio_async_result
from .widgets import EditDeviceDialog, TurnOnApplicationWindow


def _read_arp_cache(path: Path) -> list[ArpCacheEntry]:
    entries: list[ArpCacheEntry] = []
    with path.open() as source:
        for line in islice(source, 1, None):
            try:
                entries.append(ArpCacheEntry.parse(line))
            except ValueError as error:
                log.warn(f"Ignoring ARP cache entry '{line}': {error}")
    return entries


async def _reverse_lookup_device_label(
    device: DeviceObject, address: IPv4Address
) -> None:
    inetaddress = Gio.InetAddress.new_from_string(str(address))
    if inetaddress is None:
        raise ValueError(f"Failed to create inet address from {address}")
    log.info(f"Looking up name for {address}")
    resolver = Gio.Resolver.get_default()
    name = await gio_async_result(
        lambda c, cb: resolver.lookup_by_address_async(inetaddress, c, cb),
        resolver.lookup_by_address_finish,
    )
    log.info(f"Address {address} resolved to {name}")
    device.label = name


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
        self._arp_cache_file = Path("/proc/net/arp")
        self._registered_devices = Gio.ListStore[DeviceObject].new(DeviceObject)
        self._discovered_devices = Gio.ListStore[DeviceObject].new(DeviceObject)
        self._devices_file: Path = (
            Path(GLib.get_user_data_dir()) / application_id / "devices.json"
        )
        self._add_options()
        self._setup_actions()
        self._device_storage: DeviceStorage | None = None
        self._scan_network_task: asyncio.Task[None] | None = None
        self._dbus_registrations: set[int] = set()

    @GObject.Property(type=bool, default=False)
    def scan_network(self) -> bool:
        """Whether to scan the network for devices."""
        return self._scan_network_task is not None

    @scan_network.setter
    def set_scan_network(self, scan: bool) -> None:
        """Enable or disable network scanning."""
        if scan:
            task = asyncio.create_task(self._scan_network(), name="scan-network")
            task.add_done_callback(log.log_task_exception)
            self._scan_network_task = task
        elif self._scan_network_task is not None:
            self._scan_network_task.cancel()
            self._scan_network_task = None
            self._discovered_devices.remove_all()

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
            "turn-on-device",
            0,
            GLib.OptionFlags.NONE,
            GLib.OptionArg.STRING,
            C_(
                "option.turn-on-device.description",
                "Turn on a device by its label",
            ),
            C_(
                "option.turn-on-device.arg.description",
                "LABEL",
            ),
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
        self.add_main_option(
            "arp-cache-file",
            0,
            GLib.OptionFlags.HIDDEN,
            GLib.OptionArg.FILENAME,
            C_(
                "option.arp-cache-file.description",
                "Use the given file as ARP cache source (for development only)",
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

    def _new_device_saved(
        self, _dialog: EditDeviceDialog, device: DeviceObject
    ) -> None:
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
        scan_network = Gio.PropertyAction.new("scan-network", self, "scan-network")
        for action in [about, quit, add, scan_network]:
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
        self.set_accels_for_action("app.scan-network", ["F5"])

    async def _scan_network(self) -> None:
        entries = await asyncio.to_thread(
            partial(_read_arp_cache, self._arp_cache_file)
        )
        async with asyncio.TaskGroup() as reverse_lookups:
            for entry in entries:
                if entry.hardware_type != ArpHardwareType.ETHER:
                    continue
                if ArpFlag.ATF_COM not in entry.flags:
                    continue
                device = DeviceObject(
                    Device(
                        label=C_("discovered-device.label", "Discovered device"),
                        host=str(entry.ip_address),
                        mac_address=entry.hardware_address,
                        target_address=SocketAddress(
                            IPv4Interface(
                                (entry.ip_address, 24)
                            ).network.broadcast_address,
                            9,
                        ),
                    )
                )
                task = reverse_lookups.create_task(
                    _reverse_lookup_device_label(device, entry.ip_address),
                    name=f"reverse-lookup/{entry.ip_address}",
                )
                task.add_done_callback(log.log_task_exception)
                self._discovered_devices.append(device)

    def _search_provider_search_launched(
        self, _sp: SearchProvider, _terms: list[str]
    ) -> None:
        # We don't have any in app search (yet?) so just activate the app
        # to show the main window
        self.activate()

    def _search_provider_notification(
        self, _sp: SearchProvider, notification: Gio.Notification, timeout: int
    ) -> None:
        id = GLib.uuid_string_random()
        self.send_notification(id, notification)
        if 0 < timeout:

            def _withdraw() -> bool:
                self.withdraw_notification(id)
                return False

            GLib.timeout_add_seconds(timeout, _withdraw)

    @override
    def do_dbus_register(
        self, connection: Gio.DBusConnection, object_path: str
    ) -> bool:
        app_id = self.get_application_id()
        assert app_id is not None
        search_provider = SearchProvider(app_id, self._registered_devices)
        search_provider.connect(
            "search-launched", self._search_provider_search_launched
        )
        search_provider.connect("send-notification", self._search_provider_notification)
        self._dbus_registrations.add(
            DBusObject(
                "/de/swsnr/turnon/search",
                search_provider_interface(),
                search_provider.call_method,
            ).register_on(connection)
        )
        return Adw.Application.do_dbus_register(self, connection, object_path)

    @override
    def do_dbus_unregister(
        self, connection: Gio.DBusConnection, object_path: str
    ) -> None:
        Adw.Application.do_dbus_unregister(self, connection, object_path)
        for registration in self._dbus_registrations:
            connection.unregister_object(registration)
        self._dbus_registrations.clear()

    @override
    def do_handle_local_options(self, options: GLib.VariantDict) -> int:
        _ = Adw.Application.do_handle_local_options(self, options)

        devices_file = options.lookup_value("devices-file")
        if devices_file:
            self._devices_file = Path(
                devices_file.get_bytestring().decode(sys.getfilesystemencoding())
            )
            log.warn(
                f"Overriding storage file to {self._devices_file}; "
                + "only use for development purposes!",
            )

        arp_cache_file = options.lookup_value("arp-cache-file")
        if arp_cache_file:
            self._arp_cache_file = Path(
                arp_cache_file.get_bytestring().decode(sys.getfilesystemencoding())
            )
            log.warn(
                f"Overriding ARP cache file to {self._devices_file}; "
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

        turn_on_device = options.lookup_value("turn-on-device")
        if turn_on_device is not None:
            turn_on_device = turn_on_device.get_string()
            return AppCLI(
                self, self._registered_devices, command_line
            ).turn_on_device_by_label(turn_on_device)

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
            self._registered_devices.append(DeviceObject(device))

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
            window = TurnOnApplicationWindow(
                self, self._registered_devices, self._discovered_devices
            )
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
