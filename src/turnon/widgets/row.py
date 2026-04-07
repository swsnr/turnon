# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""A device row."""

from functools import partial
from typing import cast, override

from gi.repository import Adw, GLib, GObject, Gtk

from turnon.monitor import DeviceMonitor

from ..model import DeviceObject
from .edit import EditDeviceDialog
from .util import add_shortcuts


class MoveDirection(GObject.GEnum):
    """A direction in which a device was moved."""

    Upwards = 1
    Downwards = 2


@Gtk.Template.from_resource("/de/swsnr/turnon/device-row.ui")
class DeviceRow(Adw.ActionRow):
    """A action row for one device to wake up."""

    __gtype_name__ = "TurnOnDeviceRow"

    def __init__(self, device: DeviceObject, monitor_interval_s: int) -> None:
        """Initialize a device row with a `device`.

        Periodically ping the device at the given `monitor_interval_s` (seconds).
        """
        super().__init__()
        self._device = device
        self._suffix_mode = "buttons"
        self._device_monitor: DeviceMonitor | None = DeviceMonitor(device, 5)
        self.notify("device")
        self.notify("device-monitor")

    @GObject.Property(type=DeviceObject, flags=GObject.ParamFlags.READABLE)
    def device(self) -> DeviceObject:
        """Get the current the device."""
        return self._device

    @GObject.Property(type=DeviceMonitor, flags=GObject.ParamFlags.READABLE)
    def device_monitor(self) -> DeviceMonitor | None:
        """Get the monitor for the device."""
        return self._device_monitor

    @GObject.Property(type=str, default="buttons")
    def suffix_mode(self) -> str:
        """Get the suffix mode of this row."""
        return self._suffix_mode

    @suffix_mode.setter
    def set_suffix_mode(self, mode: str) -> None:
        """Set the suffix mode."""
        self._suffix_mode = mode

    @GObject.Signal(arg_types=[MoveDirection])  # pyright: ignore[reportUntypedFunctionDecorator]
    def moved(self, direction: MoveDirection) -> None:
        """Signal emitted when a device is moved in a given direction."""
        pass

    @GObject.Signal()  # pyright: ignore[reportUntypedFunctionDecorator]
    def deleted(self) -> None:
        """Signal emitted when a device is deleted."""
        pass

    @GObject.Signal()  # pyright: ignore[reportUntypedFunctionDecorator]
    def added(self, device: DeviceObject) -> None:
        """Signal emitted when a device is added as a new device."""
        pass

    @override
    def do_unroot(self) -> None:
        Adw.ActionRow.do_unroot(self)
        if self._device_monitor is not None:
            self._device_monitor.stop()
            self._device_monitor = None

    @Gtk.Template.Callback()
    @staticmethod
    def device_mac_address(_row: "DeviceRow", device: DeviceObject | None) -> str:
        """Return the formatted MAC address of a device."""
        if device:
            return str(device.mac_address)
        else:
            return ""

    @Gtk.Template.Callback()
    @staticmethod
    def device_state_name(_row: "DeviceRow", is_device_online: bool | None) -> str:
        """Return the name of the state of a device."""
        return "online" if is_device_online else "offline"

    @Gtk.Template.Callback()
    @staticmethod
    def device_host(_row: "DeviceRow", host: str, url: str | None) -> str:
        """Return the device host of this row."""
        if url:
            return (
                f'<a href="{GLib.markup_escape_text(url)}">'
                + f"{GLib.markup_escape_text(host)}</a>"
            )
        else:
            return GLib.markup_escape_text(host)


def _activate_move(
    direction: MoveDirection,
    row: Gtk.Widget,
    action: str,
    argument: GLib.Variant | None,
) -> None:
    assert isinstance(row, DeviceRow)
    row.emit("moved", direction)


def _activate_suffix_mode(
    mode: str,
    row: Gtk.Widget,
    action: str,
    argument: GLib.Variant | None,
) -> None:
    assert isinstance(row, DeviceRow)
    row.suffix_mode = mode


def _activate_delete(
    row: Gtk.Widget, action: str, argument: GLib.Variant | None
) -> None:
    assert isinstance(row, DeviceRow)
    row.emit("deleted")


def _activate_edit(row: Gtk.Widget, action: str, argument: GLib.Variant | None) -> None:
    assert isinstance(row, DeviceRow)
    dialog = EditDeviceDialog(row.device)
    dialog.present(row)


def _activate_add(row: Gtk.Widget, action: str, argument: GLib.Variant | None) -> None:
    assert isinstance(row, DeviceRow)
    new_device = DeviceObject(cast(DeviceObject, row.device).device)
    dialog = EditDeviceDialog(new_device)

    def _on_saved(dialog: EditDeviceDialog, device: DeviceObject) -> None:
        row.emit("added", device)

    dialog.connect("saved", _on_saved)
    dialog.present(row)


DeviceRow.install_action(
    "row.move-up", None, partial(_activate_move, MoveDirection.Upwards)
)
DeviceRow.install_action(
    "row.move-down", None, partial(_activate_move, MoveDirection.Downwards)
)
DeviceRow.install_action(
    "row.ask-delete", None, partial(_activate_suffix_mode, "confirm-delete")
)
DeviceRow.install_action(
    "row.cancel-delete", None, partial(_activate_suffix_mode, "buttons")
)
DeviceRow.install_action("row.delete", None, _activate_delete)
DeviceRow.install_action("row.edit", None, _activate_edit)
DeviceRow.install_action("row.add", None, _activate_add)


add_shortcuts(
    DeviceRow,
    [
        ("<Alt>Up", "action(row.move-up)"),
        ("<Alt>Down", "action(row.move-down)"),
        ("<Alt>Return", "action(row.edit)"),
        ("<Ctrl>n", "action(row.add)"),
        ("Delete", "action(row.ask-delete)"),
        ("<Ctrl>Delete", "action(row.delete)"),
        ("Escape", "action(row.cancel-delete)"),
    ],
)
