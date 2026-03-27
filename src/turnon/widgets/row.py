# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""A device row."""

from functools import partial

from gi.repository import Adw, GLib, GObject, Gtk

from ..model import Device
from .util import add_shortcuts


class MoveDirection(GObject.GEnum):
    """A direction in which a device was moved."""

    Upwards = 1
    Downwards = 2


@Gtk.Template.from_resource("/de/swsnr/turnon/device-row.ui")
class DeviceRow(Adw.ActionRow):
    """A action row for one device to wake up."""

    __gtype_name__ = "TurnOnDeviceRow"

    def __init__(self, device: Device) -> None:
        """Initialize a device row with a `device`."""
        super().__init__()
        self._device = device
        self._is_device_online: bool = False
        self._device_url: str | None = None
        self._suffix_mode = "buttons"
        self.notify("device")

    @GObject.Property(type=Device)
    def device(self) -> Device:
        """Get the current the device."""
        return self._device

    @device.setter
    def set_device(self, device: Device) -> None:
        """Set the device."""
        self._device = device

    @GObject.Property(type=bool, default=False)
    def is_device_online(self) -> bool:
        """Whether the device is online."""
        return self._is_device_online

    @is_device_online.setter
    def set_is_device_online(self, value: bool) -> None:
        """Update the online state of the device."""
        self._is_device_online = value

    @GObject.Property(type=str)
    def device_url(self) -> str | None:
        """Get the URL of the device."""
        return self._device_url

    @GObject.Property(type=str, default="buttons")
    def suffix_mode(self) -> str:
        """Get the suffix mode of this row."""
        return self._suffix_mode

    @suffix_mode.setter
    def set_suffix_mode(self, mode: str) -> None:
        """Set the suffix mode."""
        self._suffix_mode = mode

    @GObject.Signal(arg_types=[Device, MoveDirection])  # pyright: ignore[reportUntypedFunctionDecorator]
    def moved(self, device: Device, direction: MoveDirection) -> None:
        """Signal emitted when a device is moved in a given direction."""
        pass

    @GObject.Signal(arg_types=[Device])  # pyright: ignore[reportUntypedFunctionDecorator]
    def deleted(self, device: Device) -> None:
        """Signal emitted when a device is deleted."""
        pass

    @Gtk.Template.Callback()
    @staticmethod
    def device_mac_address(_row: "DeviceRow", device: Device | None) -> str:
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
    row.emit("moved", row.device, direction)


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
    row.emit("deleted", row.device)


def _activate_edit(row: Gtk.Widget, action: str, argument: GLib.Variant | None) -> None:
    assert isinstance(row, DeviceRow)
    raise NotImplementedError()


def _activate_add(row: Gtk.Widget, action: str, argument: GLib.Variant | None) -> None:
    assert isinstance(row, DeviceRow)
    raise NotImplementedError()


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
    ],
)
