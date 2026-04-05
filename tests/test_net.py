# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Pure Python model tests."""

import asyncio
import socket
import time
from asyncio import DatagramProtocol
from contextlib import closing
from ipaddress import IPv4Address, IPv6Address, ip_address
from typing import Any, cast, override

import pytest

from turnon.net import MacAddress, SocketAddress, ping_ip_address, wol


@pytest.mark.parametrize(
    "ipaddr,port,expected",
    [
        ("192.0.2.12", 42, "192.0.2.12:42"),
        ("2001:DB8::12:1", 42, "[2001:db8::12:1]:42"),
    ],
)
def test_socket_address_str(ipaddr: str, port: int, expected: str) -> None:
    """Test str formatting of socket addresses."""
    sockaddr = SocketAddress(ip_address(ipaddr), port)
    assert str(sockaddr) == expected


def test_macaddress_from_bytes() -> None:
    """Test initializing a MAC address from bytes."""
    address = b"\x11\x12\x13\x14\x15\x16"
    assert MacAddress(address).address == address


@pytest.mark.parametrize(
    "s,expected",
    [
        ("FC:3F:DB:7B:26:D7", MacAddress(b"\xfc\x3f\xdb\x7b\x26\xd7")),
        ("fc:3f:db:7b:26:d7", MacAddress(b"\xfc\x3f\xdb\x7b\x26\xd7")),
    ],
)
def test_macaddress_parse_valid(s: str, expected: MacAddress) -> None:
    """Test parsing a MAC address from a string."""
    assert MacAddress.parse(s) == expected


@pytest.mark.parametrize("address", [b"\x11\x12", b"\x11\x12\x13\x14\x15\x16\x17"])
def test_macaddress_from_invalid_bytes(address: bytes) -> None:
    """Test initializing a MAC address from invalid bytes."""
    with pytest.raises(ValueError):
        MacAddress(address)


@pytest.mark.parametrize(
    "address,expected",
    [
        (b"\x11\x12\x13\x14\x15\x16", "11:12:13:14:15:16"),
        (b"\xaa\xab\xac\xad\xae\xaf", "AA:AB:AC:AD:AE:AF"),
    ],
)
def test_macdress_str(address: bytes, expected: str) -> None:
    """Test str formatting for MAC addresses."""
    assert str(MacAddress(address)) == expected


@pytest.mark.asyncio
async def test_ping_loopback_v4() -> None:
    """Test pinging an IPv4 loopback address."""
    (address, rtt) = await ping_ip_address(IPv4Address("127.0.0.1"), sequence_number=3)
    rtt_s = rtt / 1_000_000_000
    assert rtt_s < 1
    assert address == IPv4Address("127.0.0.1")


@pytest.mark.asyncio
async def test_ping_loopback_v6() -> None:
    """Test pinging an IPv6 loopback address."""
    (address, rtt) = await ping_ip_address(IPv6Address("::1"), sequence_number=26)
    rtt_s = rtt / 1_000_000_000
    assert rtt_s < 1
    assert address == IPv6Address("::1")


@pytest.mark.asyncio
async def test_ping_unroutable_with_fast_timeout() -> None:
    """Test a timeout with an unroutable address.

    Test that pinging an unroutable address wrapped in a short `wait_for` timeout
    times out fast, instead of waiting for standard I/O timeouts; this tests that
    we don't accidentally use blocking sockets.
    """
    now = time.monotonic()
    try:
        await asyncio.wait_for(
            ping_ip_address(IPv4Address("192.0.2.42"), sequence_number=3), timeout=1
        )
    except TimeoutError:
        assert (time.monotonic() - now) <= 1.25


class _TrackDatagramsProtocol(DatagramProtocol):
    def __init__(self) -> None:
        self.datagrams = asyncio.Queue[bytes]()

    @override
    def datagram_received(self, data: bytes, addr: Any) -> None:
        self.datagrams.put_nowait(data)


@pytest.mark.asyncio
async def test_wol_send_packet() -> None:
    """Test sending a magic packet to a local socket server."""
    (transport, protocol) = await asyncio.get_event_loop().create_datagram_endpoint(
        _TrackDatagramsProtocol,
        ("127.0.0.1", 0),
        family=socket.AF_INET,
        proto=socket.IPPROTO_UDP,
    )
    (_, port) = cast(tuple[str, int], transport.get_extra_info("sockname"))
    with closing(transport):
        await wol(
            MacAddress.parse("0E-12-13-14-15-16"),
            SocketAddress(IPv4Address("127.0.0.1"), port),
        )
        data = await protocol.datagrams.get()
        assert data == b"".join(
            [
                b"\xff\xff\xff\xff\xff\xff",  # Six fill bytes
                b"\x0e\x12\x13\x14\x15\x16",  # Sixteen repetitions of the MAC address
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",  #  5
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",  # 10
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",
                b"\x0e\x12\x13\x14\x15\x16",  # 15
                b"\x0e\x12\x13\x14\x15\x16",
            ]
        )
