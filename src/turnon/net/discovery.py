# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Device discovery."""

import asyncio
from collections.abc import AsyncGenerator
from functools import partial
from gettext import pgettext as C_
from ipaddress import IPv4Address, IPv4Interface
from itertools import islice
from pathlib import Path

from gi.repository import Gio

from .. import log
from ..model import Device, DeviceObject
from ..net import SocketAddress
from ..util import gio_async_result
from .arp import ArpCacheEntry, ArpFlag, ArpHardwareType


async def _reverse_lookup_device_label(
    device: DeviceObject, address: IPv4Address
) -> None:
    inetaddress = Gio.InetAddress.new_from_string(str(address))
    if inetaddress is None:
        raise ValueError(f"Failed to create inet address from {address}")
    log.info(f"Looking up name for {address}")
    resolver = Gio.Resolver.get_default()
    name = await gio_async_result(
        lambda c, cb: resolver.lookup_by_address_async(inetaddress, c, cb),
        resolver.lookup_by_address_finish,
    )
    log.info(f"Address {address} resolved to {name}")
    device.label = name


def _read_arp_cache(path: Path) -> list[ArpCacheEntry]:
    entries: list[ArpCacheEntry] = []
    with path.open() as source:
        for line in islice(source, 1, None):
            try:
                entries.append(ArpCacheEntry.parse(line))
            except ValueError as error:
                log.warn(f"Ignoring ARP cache entry '{line}': {error}")
    return entries


async def scan_network(arp_cache_file: Path) -> AsyncGenerator[DeviceObject]:
    """Scan network for devices."""
    entries = await asyncio.to_thread(partial(_read_arp_cache, arp_cache_file))
    async with asyncio.TaskGroup() as reverse_lookups:
        for entry in entries:
            if entry.hardware_type != ArpHardwareType.ETHER:
                continue
            if ArpFlag.ATF_COM not in entry.flags:
                continue
            device = DeviceObject(
                Device(
                    label=C_("discovered-device.label", "Discovered device"),
                    host=str(entry.ip_address),
                    mac_address=entry.hardware_address,
                    target_address=SocketAddress(
                        IPv4Interface((entry.ip_address, 24)).network.broadcast_address,
                        9,
                    ),
                )
            )
            task = reverse_lookups.create_task(
                _reverse_lookup_device_label(device, entry.ip_address),
                name=f"reverse-lookup/{entry.ip_address}",
            )
            task.add_done_callback(log.log_task_exception)
            yield device
