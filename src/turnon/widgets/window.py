# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Turn On main window."""

import asyncio
from functools import partial
from gettext import pgettext as C_

from gi.repository import Adw, Gio, Gtk

from turnon.model.gobject import CombinedListModel

from .. import log, net
from ..model import Device
from .row import DeviceRow, MoveDirection


@Gtk.Template.from_resource("/de/swsnr/turnon/turnon-application-window.ui")
class TurnOnApplicationWindow(Adw.ApplicationWindow):
    """Main application window for Turn On."""

    __gtype_name__: str = "TurnOnApplicationWindow"

    devices_list: Gtk.ListBox = Gtk.Template.Child()
    feedback: Adw.ToastOverlay = Gtk.Template.Child()

    def __init__(
        self,
        application: Adw.Application,
        registered_devices: Gio.ListStore[Device],
        discovered_devices: Gio.ListModel[Device],
    ) -> None:
        """Create an application window for the given application."""
        super().__init__(application=application)
        self._registered_devices = registered_devices
        self._discovered_devices = discovered_devices
        self.devices_list.bind_model(
            CombinedListModel(
                Device, self._registered_devices, self._discovered_devices
            ),
            self._create_device_row,
        )
        self._tasks: set[asyncio.Task[None]] = set()

    def _device_deleted(self, row: DeviceRow) -> None:
        (is_registered, index) = self._registered_devices.find(row.device)
        if is_registered:
            self._registered_devices.remove(index)

    def _device_added(self, _row: DeviceRow, device: Device) -> None:
        self._registered_devices.append(device)

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

    def _send_toast(self, title: str, *, timeout: int) -> Adw.Toast:
        toast = Adw.Toast.new(title)
        toast.set_timeout(timeout)
        self.feedback.add_toast(toast)
        return toast

    async def _wakeup_device(self, device: Device) -> None:
        async with asyncio.timeout(5):
            await net.wol(device.mac_address, device.target_address)

    def _notify_wol_finished(
        self, device: Device, sent_toast: Adw.Toast, task: asyncio.Task[None]
    ) -> None:
        sent_toast.dismiss()
        if task.cancelled():
            return
        if task.exception() is None:
            self._send_toast(
                C_(
                    "application-window.feedback.toast",
                    "Sent magic packet to device {device_label}",
                ).format(device_label=device.label),
                timeout=3,
            )
        else:
            self._send_toast(
                C_(
                    "application-window.feedback.toast",
                    "Failed to send magic packet to device {device_label}",
                ).format(device_label=device.label),
                timeout=10,
            )

    def _device_activated(self, row: DeviceRow) -> None:
        device: Device = row.device
        sent_toast = self._send_toast(
            C_(
                "application-window.feedback.toast",
                "Sending magic packet to device {device_label}",
            ).format(device_label=device.label),
            timeout=3,
        )
        task = asyncio.create_task(
            self._wakeup_device(device),
            name=f"wol/{device.mac_address}/{device.target_address}/{len(self._tasks)}",
        )
        self._tasks.add(task)
        task.add_done_callback(self._tasks.discard)
        task.add_done_callback(log.log_task_exception)
        task.add_done_callback(partial(self._notify_wol_finished, device, sent_toast))

    def _create_device_row(self, device: Device) -> Gtk.Widget:
        # Ping device every 5 seconds to check whether it's online
        row = DeviceRow(device, monitor_interval_s=5)

        row.connect("deleted", self._device_deleted)
        row.connect("added", self._device_added)
        row.connect("moved", self._device_moved)
        row.connect("activated", self._device_activated)

        (is_registered, _) = self._registered_devices.find(device)
        row.action_set_enabled("row.ask-delete", is_registered)
        row.action_set_enabled("row.delete", is_registered)
        row.action_set_enabled("row.edit", is_registered)
        row.action_set_enabled("row.add", not is_registered)
        row.action_set_enabled("row.move-up", is_registered)
        row.action_set_enabled("row.move-down", is_registered)
        if not is_registered:
            row.add_css_class("discovered")

        row.device_monitor.start()

        return row


TurnOnApplicationWindow.add_shortcut(
    Gtk.Shortcut(
        action=Gtk.NamedAction.new("app.add-device"),
        trigger=Gtk.ShortcutTrigger.parse_string("<Ctrl>N"),
    )
)
