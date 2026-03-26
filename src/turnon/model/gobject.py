# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""GObject based model classes."""

import asyncio
import dataclasses

from gi.repository import Gio, GObject

from turnon.net import wol

from .. import log
from .pure import Device as PureDevice
from .pure import MacAddress, SocketAddress


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
        self._device = dataclasses.replace(self.device, address=value)

    @GObject.Property()
    def target_address(self) -> SocketAddress:
        """Get the target address for this device."""
        return self._device.target_address

    @target_address.setter
    def set_target_address(self, value: SocketAddress) -> None:
        """Set the target address for this device."""
        self._device = dataclasses.replace(self.device, target_address=value)

    async def wol(self) -> None:
        """Wake up this device."""
        mac_address = self._device.mac_address
        target_address = self._device.target_address
        log.info(
            f"Sending magic packet for mac address {mac_address} of device "
            + f"{self._device.label} to {target_address}"
        )
        timeout_s = 5
        try:
            await asyncio.wait_for(wol(mac_address, target_address), timeout_s)
            log.info(
                f"Sent magic packet to {mac_address} "
                + f"of device {self._device.label} to {target_address}"
            )
        except Exception as error:
            log.warn(
                "Failed to send magic packet to {mac_address} "
                + f"of device {self._device.label} to {target_address}: {error}",
            )
            raise


class DeviceDiscovery(GObject.Object, Gio.ListModel[Device]):
    """Discover devices."""

    # TODO

    def do_get_item_type(self) -> type[Device]:
        """Get the type of items in this list model."""
        return Device

    def do_get_n_items(self) -> int:
        """Return number of items in this list model."""
        return 0

    def do_get_item(self, position: int) -> Device | None:
        """Get the device at the given `position`."""
        return None


class Devices(GObject.Object, Gio.ListModel[Device]):
    """A list model for devices."""

    __gtype_name__ = "TurnOnDevices"

    def __init__(self) -> None:
        """Create a new list of devices."""
        super().__init__()

        # The list of devices the user has registered
        self._registered_devices = Gio.ListStore[Device](item_type=Device)

        # The list of devices discovered from network scanning.
        self._discovered_devices = DeviceDiscovery()

        self._registered_devices.connect(
            "items-changed", self._registered_devices_changed
        )
        self._discovered_devices.connect(
            "items-changed", self._discovered_devices_changed
        )

    @GObject.Property(type=Gio.ListStore)
    def registered_devices(self) -> Gio.ListStore[Device]:
        """Get devices added by the user."""
        return self._registered_devices

    @GObject.Property(type=DeviceDiscovery)
    def discovered_devices(self) -> DeviceDiscovery:
        """Get devices discovered in the network."""
        return self._discovered_devices

    def _registered_devices_changed(
        self, _store: Gio.ListStore[Device], position: int, removed: int, added: int
    ) -> None:
        self.emit("items-changed", position, removed, added)

    def _discovered_devices_changed(
        self, _devices: DeviceDiscovery, position: int, removed: int, added: int
    ) -> None:
        self.emit(
            "items-changed",
            position + self._registered_devices.get_n_items(),
            removed,
            added,
        )

    def do_get_item_type(self) -> type[Device]:
        """Get the type of items in this list model."""
        return Device

    def do_get_n_items(self) -> int:
        """Return number of items in this list model."""
        return (
            self._registered_devices.get_n_items()
            + self._discovered_devices.get_n_items()
        )

    def do_get_item(self, position: int) -> Device | None:
        """Get the device at the given `position`."""
        n_registered = self._registered_devices.get_n_items()
        if position < n_registered:
            return self._registered_devices.get_item(position)
        else:
            return self._discovered_devices.get_item(position - n_registered)
