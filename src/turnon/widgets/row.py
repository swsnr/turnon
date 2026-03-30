# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""A device row."""

import asyncio
from functools import partial
from ipaddress import ip_address
from typing import override

from gi.repository import Adw, GLib, GObject, Gtk

from .. import log
from ..model import Device
from ..net import monitor
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

    def __init__(self, device: Device) -> None:
        """Initialize a device row with a `device`."""
        super().__init__()
        self._device = device
        self._is_device_online: bool = False
        self._device_url: str | None = None
        self._suffix_mode = "buttons"
        self.notify("device")
        self._monitor_task: asyncio.Task[None] | None = None
        self._device.connect("notify::host", self._device_host_changed)

    @GObject.Property(type=Device, flags=GObject.PARAM_READABLE)
    def device(self) -> Device:
        """Get the current the device."""
        return self._device

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

    def _device_host_changed(self, _device: Device, _prop: str) -> None:
        if self._monitor_task and not self._monitor_task.cancelled():
            log.message("Restarting monitor")
            self.stop_monitoring()
            self.start_monitoring()

    async def _monitor_host(self) -> None:
        host = self._device.host
        try:
            address = ip_address(host)
        except ValueError:
            log.warn("Monitoring hosts is not yet supported")
            return
        async for result in monitor(address, interval=5):
            if result is None:
                log.message(f"{address} not reachable")
                self.is_device_online = False
            else:
                (_, rtt) = result
                log.message(f"{address} replied after {rtt}s")
                self.is_device_online = True

    def start_monitoring(self) -> None:
        """Start monitoring the device."""
        if self._monitor_task and not self._monitor_task.cancelled():
            return
        self._monitor_task = asyncio.create_task(self._monitor_host())

    def stop_monitoring(self) -> None:
        """Stop monitoring this device."""
        if self._monitor_task:
            self._monitor_task.cancel()
            self._monitor_task = None

    @GObject.Signal(arg_types=[MoveDirection])  # pyright: ignore[reportUntypedFunctionDecorator]
    def moved(self, direction: MoveDirection) -> None:
        """Signal emitted when a device is moved in a given direction."""
        pass

    @GObject.Signal()  # pyright: ignore[reportUntypedFunctionDecorator]
    def deleted(self) -> None:
        """Signal emitted when a device is deleted."""
        pass

    @override
    def do_unroot(self) -> None:
        Adw.ActionRow.do_unroot(self)
        self.stop_monitoring()

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
        ("Escape", "action(row.cancel-delete)"),
    ],
)
