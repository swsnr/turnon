# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""GObject based model classes."""

import dataclasses
from pathlib import Path
from threading import Condition, Thread
from typing import override

from gi.repository import Gio, GObject

from .. import log
from .pure import Device as PureDevice
from .pure import MacAddress, SocketAddress
from .storage import dump_devices


class Device(GObject.Object):
    """A GObject-based device which wraps a pure-python device model."""

    __gtype_name__ = "TurnOnDevice"

    def __init__(self, device: PureDevice) -> None:
        """Initialize a new device."""
        super().__init__()
        self._device = device

    @property
    def device(self) -> PureDevice:
        """Get the underlying pure device.

        This is not a GObject property.
        """
        return self._device

    @GObject.Property(type=str)
    def label(self) -> str:
        """Get the label of this device."""
        return self._device.label

    @label.setter
    def set_label(self, value: str) -> None:
        """Set the label of this device."""
        self._device = dataclasses.replace(self.device, label=value)

    @GObject.Property(type=str)
    def host(self) -> str:
        """Get the host name of this device."""
        return self._device.host

    @host.setter
    def set_host(self, value: str) -> None:
        """Set the host name of this device."""
        self._device = dataclasses.replace(self.device, host=value)

    @GObject.Property()
    def mac_address(self) -> MacAddress:
        """Get the MAC address of this device."""
        return self._device.mac_address

    @mac_address.setter
    def set_mac_address(self, value: MacAddress) -> None:
        """Set the MAC address of this device."""
        self._device = dataclasses.replace(self.device, mac_address=value)

    @GObject.Property()
    def target_address(self) -> SocketAddress:
        """Get the target address for this device."""
        return self._device.target_address

    @target_address.setter
    def set_target_address(self, value: SocketAddress) -> None:
        """Set the target address for this device."""
        self._device = dataclasses.replace(self.device, target_address=value)


class DeviceStorage(Thread):
    """Automatically save devices."""

    def __init__(self, path: Path) -> None:
        """Create a new device storage thread."""
        super().__init__(name="save-devices-automatically", daemon=False)
        self._path = path
        self._save_condition = Condition()
        self._devices_to_save: list[PureDevice] | None = None
        self._stop = False

    @override
    def run(self) -> None:
        while True:
            devices = None

            # Critical section: wait until we're notified, and then check whether
            # we've got to stop or have to save devices, but don't actually do any I/O
            # here.
            with self._save_condition:
                self._save_condition.wait()
                if self._stop:
                    return
                devices = self._devices_to_save
                self._devices_to_save = None

            if devices:
                log.message(f"Saving {len(devices)} device(s) to {self._path}")
                # Save devices but outside of our critical section.
                dump_devices(self._path, devices)

    def request_stop(self) -> None:
        """Request that this thread stops."""
        with self._save_condition:
            self._stop = True
            self._save_condition.notify_all()

    def request_save_devices(self, devices: list[PureDevice]) -> None:
        """Request that this thread should save some devices."""
        with self._save_condition:
            self._devices_to_save = devices
            self._save_condition.notify_all()

    def _devices_changed(
        self, devices: Gio.ListModel[Device], position: int, removed: int, added: int
    ) -> None:
        if 0 < added:
            # Monitor all new devices for changes
            for i in range(position, devices.get_n_items()):
                device = devices.get_item(i)
                assert device
                device.connect(
                    "notify",
                    lambda *args: self.request_save_devices(
                        [d.device for d in devices]
                    ),
                )

        # Save devices
        self.request_save_devices([d.device for d in devices])

    def save_automatically(self, devices: Gio.ListModel[Device]) -> None:
        """Monitor `devices` and save automatically on changes."""
        devices.connect("items-changed", self._devices_changed)
        for device in devices:
            device.connect(
                "notify",
                lambda *args: self.request_save_devices([d.device for d in devices]),
            )


class CombinedListModel[T: GObject.Object](GObject.Object, Gio.ListModel[T]):
    """A list model which combines two list models."""

    def __init__(
        self, item_type: type[T], model1: Gio.ListModel[T], model2: Gio.ListModel[T]
    ) -> None:
        """Create a new model combining `model1` and `model2`."""
        super().__init__()
        self._item_type = item_type
        self._model1 = model1
        self._model2 = model2
        model1.connect("items-changed", self._model1_items_changed)
        model2.connect("items-changed", self._model2_items_changed)

    def _model1_items_changed(
        self, _model: Gio.ListModel[T], position: int, removed: int, added: int
    ) -> None:
        self.emit("items-changed", position, removed, added)

    def _model2_items_changed(
        self, _model: Gio.ListModel[T], position: int, removed: int, added: int
    ) -> None:
        self.emit(
            "items-changed", position + self._model1.get_n_items(), removed, added
        )

    def do_get_item(self, position: int) -> T | None:
        """Get the item at `position`."""
        model1_n = self._model1.get_n_items()
        if 0 <= position < model1_n:
            return self._model1.get_item(position)
        elif model1_n <= position:
            return self._model2.get_item(position - model1_n)

    def do_get_item_type(self) -> type[T]:
        """Get item type."""
        return self._item_type

    def do_get_n_items(self) -> int:
        """Get number of items."""
        return self._model1.get_n_items() + self._model2.get_n_items()
