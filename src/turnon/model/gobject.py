# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""GObject based model classes."""

import asyncio
import dataclasses

from gi.repository import GObject

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
