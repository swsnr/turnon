# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Networking for Turn On."""

import asyncio
import errno
import re
import socket
import struct
import time
from dataclasses import dataclass
from ipaddress import IPv4Address, IPv6Address
from itertools import chain, repeat
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


async def _ping_sockaddr(
    sockaddr: tuple[str, int] | tuple[str, int, int, int] | tuple[int, bytes],
    family: int,
    sequence_number: int,
) -> int:
    # Assemble an ICMP packet.  Luckily, ICMPv4 and ICMPv6 have the same layout
    # for echo requests, so we can use the same packet for both.
    #
    # Documentation around unprivileged ICMP is somewhat sparse in Linux land, but
    # it seems that the kernel handles the checksum and the identifier for us,
    # so we can statically assemble the packet.
    payload_line = b"turnon-ping\n"
    received_header = struct.pack(
        "!BBHHH",
        128 if family == socket.AF_INET6 else 8,
        0,  # Type (0 for ICMP echo request)
        0,  # Checksum (the kernel handles this for us)
        0,  # Identifier (kernel does this too)
        sequence_number,
    )
    payload = bytes(chain.from_iterable(repeat(payload_line, 4)))
    packet = received_header + payload

    with socket.socket(
        family=family,
        type=socket.SOCK_DGRAM,
        proto=socket.IPPROTO_ICMPV6
        if family == socket.AF_INET6
        else socket.IPPROTO_ICMP,
    ) as icmp_socket:
        icmp_socket.setblocking(False)
        loop = asyncio.get_event_loop()
        time_sent = time.monotonic_ns()
        bytes_sent = await loop.sock_sendto(icmp_socket, packet, sockaddr)
        if bytes_sent != len(packet):
            raise OSError(errno.EPIPE, f"ICMP packet to {sockaddr} not sent completely")
        # We receive the same number of bytes as we sent: The ICMP header has the same
        # size and we get our payload mirrored
        (response, _) = await loop.sock_recvfrom(icmp_socket, len(packet))
    rtt = time.monotonic_ns() - time_sent
    (received_header, received_payload) = (response[:8], response[8:])
    (response_type, _, _, _, received_seq_number) = struct.unpack(
        "!BBHHH", received_header
    )
    if response_type != (129 if family == socket.AF_INET6 else 0):
        raise OSError(errno.EBADMSG, f"Unexpected response type {response_type}")
    if received_seq_number != sequence_number:
        raise OSError(
            errno.EBADMSG,
            "Mismatched sequence number: "
            + f"expected {sequence_number}, got {received_seq_number}",
        )
    if received_payload != payload:
        raise OSError(errno.EBADMSG, "Unexpected payload received")
    return rtt


async def ping_ip_address(
    target: IPv4Address | IPv6Address, sequence_number: int
) -> int:
    """Ping a `target` address.

    Use `sequence_number` as the sequence number for the ICMP packet.

    Return the  and return
    the number of nanoseconds between sending the ping and receiving the reply.

    If the target does not reply this method will not return.  Make sure to wrap
    it in a timeout.
    """
    family = socket.AF_INET6 if isinstance(target, IPv6Address) else socket.AF_INET
    addrs = socket.getaddrinfo(
        host=str(target),
        port=0,
        family=family,
        type=socket.SOCK_DGRAM,
        flags=socket.AI_NUMERICHOST,  # Don't do DNS resolution, just resolve IP address
    )
    if not addrs:
        raise OSError(f"Failed to resolve {target}")
    elif len(addrs) == 1:
        (_, _, _, _, sockaddr) = addrs[0]
        return await _ping_sockaddr(
            sockaddr=sockaddr,
            family=family,
            sequence_number=sequence_number,
        )
    else:
        async with asyncio.TaskGroup() as pings:
            (done, _) = await asyncio.wait(
                (
                    pings.create_task(
                        _ping_sockaddr(
                            sockaddr, family=family, sequence_number=sequence_number
                        )
                    )
                    for (_, _, _, _, sockaddr) in addrs
                ),
                return_when=asyncio.FIRST_COMPLETED,
            )
            return await next(iter(done))


async def wol(mac_address: MacAddress, target_address: SocketAddress) -> None:
    """Send a magic Wake On LAN packet.

    Send a magic packet to wake the device with the given `mac_address` to
    `target_address`.
    """
    raise NotImplementedError()
