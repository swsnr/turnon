# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Pure Python model tests."""

from ipaddress import ip_address

import pytest

from turnon.net import MacAddress, SocketAddress


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
