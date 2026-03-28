# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Turn On main window."""

from gi.repository import Adw, Gio, Gtk

from ..model import Device
from .row import DeviceRow


@Gtk.Template.from_resource("/de/swsnr/turnon/turnon-application-window.ui")
class TurnOnApplicationWindow(Adw.ApplicationWindow):
    """Main application window for Turn On."""

    __gtype_name__: str = "TurnOnApplicationWindow"

    devices_list: Gtk.ListBox = Gtk.Template.Child()

    def __init__(
        self, application: Adw.Application, registered_devices: Gio.ListStore[Device]
    ) -> None:
        """Create an application window for the given application."""
        super().__init__(application=application)
        self._registered_devices = registered_devices
        self.devices_list.bind_model(registered_devices, self._create_device_row)

    def _device_deleted(self, row: DeviceRow) -> None:
        # TODO: Delete monitor
        (is_registered, index) = self._registered_devices.find(row.device)
        if is_registered:
            self._registered_devices.remove(index)

    def _create_device_row(self, device: Device) -> Gtk.Widget:
        row = DeviceRow(device)

        row.connect("deleted", self._device_deleted)

        (is_registered, _) = self._registered_devices.find(device)
        row.action_set_enabled("row.ask-delete", is_registered)
        row.action_set_enabled("row.delete", is_registered)
        row.action_set_enabled("row.edit", is_registered)
        row.action_set_enabled("row.add", not is_registered)
        row.action_set_enabled("row.move-up", is_registered)
        row.action_set_enabled("row.move-down", is_registered)
        if not is_registered:
            row.add_css_class("discovered")
        return row


TurnOnApplicationWindow.add_shortcut(
    Gtk.Shortcut(
        action=Gtk.NamedAction.new("app.add-device"),
        trigger=Gtk.ShortcutTrigger.parse_string("<Ctrl>N"),
    )
)
