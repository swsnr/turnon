# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""ARP cache access."""

from dataclasses import dataclass
from enum import Enum, Flag
from ipaddress import IPv4Address
from typing import Self

from . import MacAddress


class ArpHardwareType(Enum):
    """An ARP hardware type.

       See <https://github.com/torvalds/linux/blob/v6.12/include/uapi/linux/if_arp.h#L29>
    for known hardware types as of Linux 6.12.

    We do not represent all hardware types, but only those we're interested in
    with regards to Turn On.
    """

    # Ethernet (including WiFi)
    ETHER = 0x01


class ArpFlag(Flag):
    """Flags for ARP cache entries.

    See <https://github.com/torvalds/linux/blob/v6.12/include/uapi/linux/if_arp.h#L132>
    for known flags as of Linux 6.12.
    """

    # completed entry (ha valid)
    ATF_COM = 0x02
    # permanent entry
    ATF_PERM = 0x04
    # publish entry
    ATF_PUBL = 0x08
    # has requested trailers
    ATF_USETRAILERS = 0x10
    # want to use a netmask (only for proxy entries)
    ATF_NETMASK = 0x20
    # don't answer this addresses
    ATF_DONTPUB = 0x40


@dataclass(frozen=True, kw_only=True)
class ArpCacheEntry:
    """An entry in the ARP cache."""

    ip_address: IPv4Address
    hardware_type: ArpHardwareType | int
    flags: ArpFlag
    hardware_address: MacAddress

    @classmethod
    def parse(cls, s: str) -> Self:
        """Parse a single cache entry from a string."""
        parts = s.strip().split()
        if len(parts) < 4:
            raise ValueError(f"Unexpected number of fields: {len(parts)}")
        return cls(
            ip_address=IPv4Address(parts[0]),
            hardware_type=ArpHardwareType(int(parts[1], 16)),
            flags=ArpFlag(int(parts[2], 16)),
            hardware_address=MacAddress.parse(parts[3]),
        )
