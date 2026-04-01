# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Pure Python model classes."""

from dataclasses import dataclass

from ..net import MacAddress, SocketAddress


@dataclass(frozen=True, kw_only=True)
class Device:
    """A device that can be woken up."""

    label: str
    host: str
    mac_address: MacAddress
    target_address: SocketAddress
