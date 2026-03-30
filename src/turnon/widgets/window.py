# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Turn On main window."""

from gi.repository import Adw, Gio, Gtk

from ..model import Device
from .row import DeviceRow, MoveDirection


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
        (is_registered, index) = self._registered_devices.find(row.device)
        if is_registered:
            self._registered_devices.remove(index)

    def _device_moved(self, row: DeviceRow, direction: MoveDirection) -> None:
        (found, device_index) = self._registered_devices.find(row.device)
        if not found:
            return
        match direction:
            case MoveDirection.Upwards:
                offset = -1
            case MoveDirection.Downwards:
                offset = 1
        swap_index = device_index + offset
        swap_device = self._registered_devices.get_item(swap_index)
        if not swap_device:
            return
        # We remove the other device, not the device being moved; this
        # retains the widget for the device being moved in views consuming
        # the model, meaning it remains focused, and we can repeatedly
        # move the same device to rearrange it.
        self._registered_devices.remove(swap_index)
        self._registered_devices.insert(device_index, swap_device)

    def _create_device_row(self, device: Device) -> Gtk.Widget:
        row = DeviceRow(device)

        row.connect("deleted", self._device_deleted)
        row.connect("moved", self._device_moved)

        (is_registered, _) = self._registered_devices.find(device)
        row.action_set_enabled("row.ask-delete", is_registered)
        row.action_set_enabled("row.delete", is_registered)
        row.action_set_enabled("row.edit", is_registered)
        row.action_set_enabled("row.add", not is_registered)
        row.action_set_enabled("row.move-up", is_registered)
        row.action_set_enabled("row.move-down", is_registered)
        if not is_registered:
            row.add_css_class("discovered")

        row.start_monitoring()
        return row


TurnOnApplicationWindow.add_shortcut(
    Gtk.Shortcut(
        action=Gtk.NamedAction.new("app.add-device"),
        trigger=Gtk.ShortcutTrigger.parse_string("<Ctrl>N"),
    )
)
