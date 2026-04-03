# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Gio-based networking."""

from collections.abc import Awaitable
from ipaddress import IPv4Address, IPv6Address, ip_address
from typing import cast

from gi.repository import Gio


def _to_ip_address(address: Gio.InetAddress) -> IPv4Address | IPv6Address:
    """Convert a Gio internet address to an IP address."""
    match address.get_family():
        case Gio.SocketFamily.IPV4 | Gio.SocketFamily.IPV6:
            return ip_address(address.to_string())
        case family:
            raise ValueError(f"{address} has unsupported family: {family.value_name}")


async def lookup_host(hostname: str) -> list[IPv4Address | IPv6Address]:
    """Asynchronously lookup the given `hostname` through Gio."""
    ip_addresses = await cast(
        # Need to cast manually, see https://github.com/pygobject/pygobject-stubs/issues/220
        Awaitable[list[Gio.InetAddress]],
        Gio.Resolver.get_default().lookup_by_name_async(hostname),
    )
    # Smoke test for the cast before
    assert isinstance(ip_addresses, list)
    return [_to_ip_address(a) for a in ip_addresses]
