# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Networking for Turn On."""

import re
from dataclasses import dataclass
from ipaddress import IPv4Address, IPv6Address
from typing import Self

MAC_ADDRESS_RE = re.compile(
    r"\A[0-9a-fA-F]{2}(?P<sep>:|-)(?:(?:[0-9a-fA-F]{2})(?P=sep)){4}[0-9a-fA-F]{2}\Z"
)


@dataclass(frozen=True)
class SocketAddress:
    """A socket address."""

    address: IPv4Address | IPv6Address
    port: int

    def __str__(self) -> str:
        """Format a socket address as string."""
        if isinstance(self.address, IPv6Address):
            return f"[{self.address}]:{self.port}"
        else:
            return f"{self.address}:{self.port}"

    @classmethod
    def parse(cls, s: str) -> Self:
        """Parse a socket address from the string `s`."""
        addr, _, port = s.rpartition(":")
        try:
            port = int(port)
        except ValueError as error:
            raise ValueError(f"Invalid port {port}") from error
        if addr.startswith("["):
            if not addr.endswith("]"):
                raise ValueError(f"Invalid address {addr}")
            addr = IPv6Address(addr.lstrip("[").rstrip("]"))
        else:
            addr = IPv4Address(addr)
        return cls(addr, port)


@dataclass(frozen=True, init=False)
class MacAddress:
    """A MAC address."""

    address: bytes

    def __init__(self, address: bytes, /) -> None:
        """Initialize a MAC address directly from raw bytes.

        Raises a value error if `address` does not contain exactly six bytes.
        """
        super().__init__()
        if len(address) != 6:
            raise ValueError(f"Invalid length {len(address)}")
        object.__setattr__(self, "address", address)

    def __str__(self) -> str:
        """Format MAC address as human-readable hex string."""
        return self.address.hex(":", 1).upper()

    @staticmethod
    def is_mac_address(s: str) -> bool:
        """Whether the given string is a valid MAC address."""
        return bool(MAC_ADDRESS_RE.match(s))

    @classmethod
    def parse(cls, s: str) -> Self:
        """Parse a MAC address from string.

        Raise `ValueError` if `s` is not a valid MAC address.
        """
        match = MAC_ADDRESS_RE.match(s)
        if not match:
            raise ValueError(f"Invalid MAC address: {s}")
        sep = match["sep"]
        assert isinstance(sep, str)

        return cls(bytes.fromhex(s.replace(sep, "")))


async def wol(mac_address: MacAddress, target_address: SocketAddress) -> None:
    """Send a magic Wake On LAN packet.

    Send a magic packet to wake the device with the given `mac_address` to
    `target_address`.
    """
    raise NotImplementedError()
