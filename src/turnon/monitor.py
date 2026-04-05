# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Device monitoring."""

import asyncio
import contextlib
from ipaddress import IPv4Address, IPv6Address, ip_address
from itertools import count

from gi.repository import Gio, GLib, GObject

from . import log
from .model import DeviceObject
from .net import SocketAddress, ping_first_reachable, ping_ip_address, probe_tcp_port
from .net.gio import lookup_host


class DeviceMonitor(GObject.Object):
    """Monitor a device."""

    __gtype_name__: str = "TurnOnDeviceMonitor"

    def __init__(self, device: DeviceObject, interval: float) -> None:
        """Initialize a monitor for `device`."""
        super().__init__()
        self._device = device
        self._device.connect("notify::host", self._device_host_changed)
        self._device_online = False
        self._device_url: str | None = None
        self._interval = interval
        self._timeout = interval / 2
        self._tasks: set[asyncio.Task[None]] = set()

    @GObject.Property(type=bool, default=False, flags=GObject.ParamFlags.READABLE)
    def device_online(self) -> bool:
        """Whether the device is online."""
        return self._device_online

    def _set_device_online(self, is_online: bool) -> None:
        if self._device_online != is_online:
            self._device_online = is_online
            self.notify("device-online")
            # Clear device URL if the online state changed.
            # If the device is offline there's no URL anymore, and otherwise we
            # need to probe again.
            self._set_device_url(None)

    @GObject.Property(type=str, flags=GObject.ParamFlags.READABLE)
    def device_url(self) -> str | None:
        """Get the URL for the device if any."""
        return self._device_url

    def _set_device_url(self, url: str | None) -> None:
        self._device_url = url
        self.notify("device-url")

    def stop(self) -> None:
        """Stop monitoring."""
        for task in self._tasks:
            task.cancel()
        self._tasks.clear()

    def start(self) -> None:
        """Start monitoring."""
        self._start(delay=None)

    def _device_host_changed(self, _obj: DeviceObject, _prop: str) -> None:
        if self._tasks:
            self.stop()
            self.start()

    def _restart_on_exception(self, task: asyncio.Task[None]) -> None:
        if task.cancelled():
            return
        if task.exception():
            self._start(self._interval)

    async def _probe_http(self, address: IPv4Address | IPv6Address) -> None:
        """Probe whether `address` serves a HTTP or HTTPS.

        `address` is the IP address to test, and `host` is the host name to use for the
        HTTP URL.
        """
        for scheme, port in [("https", 443), ("http", 80)]:
            with contextlib.suppress(TimeoutError):
                async with asyncio.timeout(self._timeout):
                    log.info(f"Probing port {port} on {address}")
                    if await probe_tcp_port(SocketAddress(address, port)):
                        url = f"{scheme}://{self._device.host}:{port}"
                        log.info(f"Discovered device URL {url}")
                        self._set_device_url(url)
                        break
        else:
            self._set_device_url(None)

    def _start(self, delay: float | None) -> None:
        """Start monitoring after an optional initial `delay`.

        Create a task to periodically ping the the current device host.
        """
        host = self._device.device.host
        with contextlib.suppress(ValueError):
            host = ip_address(host)
        task = asyncio.create_task(
            self._monitor_host(host, delay=delay), name=f"monitor/{host}"
        )
        task.add_done_callback(self._tasks.discard)
        task.add_done_callback(log.log_task_exception)
        task.add_done_callback(self._restart_on_exception)
        self._tasks.add(task)

    async def _monitor_address(
        self,
        address: IPv4Address | IPv6Address,
        *,
        initial_seqnr: int,
        abort_if_unreachable: bool,
    ) -> None:
        for seqnr in count(initial_seqnr):
            try:
                # If the device is not online yet, we should probe for HTTP if
                # it comes online now.
                should_probe_http = not self._device_online
                async with asyncio.timeout(self._timeout):
                    (_, rtt) = await ping_ip_address(address, sequence_number=seqnr)
                rtt_ms = rtt / 1_000_000
                log.info(
                    f"Address {address} replied after {rtt_ms}ms (seq. nr {seqnr})"
                )
                self._set_device_online(True)
                if should_probe_http:
                    probe = asyncio.create_task(
                        self._probe_http(address), name=f"probe-http/{address}"
                    )
                    probe.add_done_callback(self._tasks.discard)
                    probe.add_done_callback(log.log_task_exception)
                    self._tasks.add(probe)
            except TimeoutError:
                self._set_device_online(False)

            await asyncio.sleep(self._interval)

            if not self._device_online and abort_if_unreachable:
                return

    async def _monitor_host(
        self, host: IPv4Address | IPv6Address | str, delay: float | None
    ) -> None:
        if delay is not None:
            await asyncio.sleep(delay)
        while True:
            try:
                seqnr = 1
                if isinstance(host, str):
                    async with asyncio.timeout(self._timeout):
                        try:
                            addresses = await lookup_host(host)
                            log.info(f"Resolved {host} to {addresses}")
                        except GLib.Error as error:
                            if (
                                GLib.quark_from_string(error.domain)
                                == Gio.ResolverError.quark()
                            ):
                                log.info(f"Failed to resolve host: {error.message}")
                                self._set_device_online(False)
                                await asyncio.sleep(self._interval)
                                continue
                            else:
                                raise
                else:
                    addresses = [host]
                assert addresses
                if len(addresses) == 1:
                    address = addresses[0]
                else:
                    # Ping all addresses once and take the one which responds first, and
                    # start monitoring that. If no address responds, wait and try to
                    # resolve again.
                    async with asyncio.timeout(self._timeout):
                        (address, rtt) = await ping_first_reachable(
                            addresses, sequence_number=seqnr
                        )
                    rtt_ms = rtt / 1_000_000
                    log.info(
                        f"Address {address} of host {host} replied first after "
                        + f"{rtt_ms}ms (seq. nr {seqnr}), monitoring {address}"
                    )
                    seqnr += 1
            except TimeoutError:
                self._set_device_online(False)
                await asyncio.sleep(self._interval)
                continue

            # Bail out if we're pinging a DNS name and it doesn't respond;
            # this makes us try to resolve the name again, which is required
            # to pick up changes in the address (e.g. for mDNS names)
            await self._monitor_address(
                address,
                initial_seqnr=seqnr,
                abort_if_unreachable=isinstance(host, str),
            )
