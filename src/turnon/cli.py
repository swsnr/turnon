# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""TurnOn command line interface."""

import asyncio
import os
from collections.abc import Coroutine, Generator
from contextlib import contextmanager, suppress
from functools import partial
from gettext import pgettext as C_
from ipaddress import IPv4Address, IPv6Address, ip_address
from typing import Any

from gi.repository import Adw, Gio, GLib

from . import log
from .model import Device, DeviceObject
from .net import ping_first_reachable, wol
from .net.gio import lookup_host


async def _resolve_host(host: str) -> list[IPv4Address | IPv6Address]:
    with suppress(ValueError):
        return [ip_address(host)]
    return await lookup_host(host)


@contextmanager
def hold_app(app: Gio.Application) -> Generator[None]:
    """Hold on to an application while running a block."""
    app.hold()
    yield
    app.release()


class AppCLI:
    """CLI for the TurnOn application."""

    def __init__(
        self,
        app: Adw.Application,
        devices: Gio.ListModel[DeviceObject],
        command_line: Gio.ApplicationCommandLine,
    ) -> None:
        """Create a new app CLI."""
        self._app = app
        self._tasks: set[asyncio.Task[None]] = set()
        self._devices = devices
        self._command_line = command_line

    def _cli_task_done(self, task: asyncio.Task[None]) -> None:
        if task.cancelled() or task.exception():
            self._command_line.set_exit_status(os.EX_OSERR)
        else:
            self._command_line.set_exit_status(os.EX_OK)
        self._command_line.done()
        self._app.release()

    def _create_cli_task(
        self, f: Coroutine[Any, Any, None], *, name: str
    ) -> asyncio.Task[None]:
        task = asyncio.create_task(f, name=name)
        self._app.hold()
        self._tasks.add(task)
        task.add_done_callback(self._tasks.discard)
        task.add_done_callback(self._cli_task_done)
        task.add_done_callback(log.log_task_exception)
        return task

    async def _ping_and_list(self) -> None:
        pings = await asyncio.gather(
            *[
                asyncio.wait_for(
                    ping_first_reachable(
                        await _resolve_host(device.host), sequence_number=1
                    ),
                    timeout=0.5,
                )
                for device in self._devices
            ],
            return_exceptions=True,
        )
        label_width = max(len(d.label) for d in self._devices)
        for device, result in zip(self._devices, pings, strict=True):
            if isinstance(result, BaseException):
                color = "\x1b[1;31m"
                indicator = "    ●"
            else:
                (_, rtt) = result
                color = "\x1b[1;32m"
                rtt_ms = round(rtt / 1_000_000)
                indicator = f"{rtt_ms:>3}ms"
            self._command_line.print_literal(
                f"{color}{indicator}\x1b[0m {device.label:<{label_width}}"
                + f"\t{device.mac_address}\t{device.host}\n"
            )

    def _wakeup_error_message(self, device: Device, task: asyncio.Task[None]) -> None:
        if task.cancelled():
            return
        exception = task.exception()
        if exception is not None:
            if isinstance(exception, TimeoutError):
                error = str(exception) or "Time out"
            elif isinstance(exception, GLib.Error):
                error = exception.message
            else:
                error = str(exception)
            self._command_line.printerr_literal(
                C_(
                    "option.turn-on-device.error",
                    "Failed to turn on device {device_label}: {error}\n",
                ).format(device_label=device.label, error=error)
            )

    async def _wakeup(self, device: Device) -> None:
        async with asyncio.timeout(5):
            await wol(device.mac_address, device.target_address)
            self._command_line.print_literal(
                C_(
                    "option.turn-on-device.message",
                    "Sent magic packet to mac address {device_mac_address} "
                    + "of device {device_label}\n",
                ).format(
                    device_mac_address=device.mac_address,
                    device_label=device.label,
                )
            )

    def list_devices(self) -> int:
        """List devices."""
        if 0 < self._devices.get_n_items():
            self._create_cli_task(self._ping_and_list(), name="cli/list-devices")
        return os.EX_OK

    def turn_on_device_by_label(self, label: str) -> int:
        """Turn on a device by its label."""
        device = next((d.device for d in self._devices if d.label == label), None)
        if device is None:
            self._command_line.printerr_literal(
                C_(
                    "option.turn-on-device.error",
                    "No device found for label {label}\n",
                )
            )
            return os.EX_DATAERR
        else:
            task = self._create_cli_task(self._wakeup(device), name="cli/add-device")
            task.add_done_callback(partial(self._wakeup_error_message, device))
        return os.EX_OK
