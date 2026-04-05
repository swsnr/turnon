# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12


"""Store and load lists of devices in JSON files."""

import json
from os import PathLike
from pathlib import Path
from threading import Condition, Thread
from typing import cast, override

from gi.repository import Gio

from .. import log
from ..net import MacAddress, SocketAddress
from .data import Device
from .gobject import DeviceObject

type Json = bool | int | str | float | list[Json] | dict[str, Json] | None


def _get_str(item: dict[str, Json], key: str) -> str:
    """Get a string at `item[key]`.

    Raise `ValueError` if `key` is missing or if its value is not a string.
    """
    value = item.get(key)
    if value is None:
        raise ValueError(f"Missing key {key}")
    if not isinstance(value, str):
        raise ValueError(
            f"Invalid value for key {key}, expected str, but got {type(value).__name__}"
        )
    return value


def _get_device(n: int, item: Json) -> Device:
    if not isinstance(item, dict):
        raise ValueError(
            f"Invalid item at position {n}: expected dict, "
            + f"got type {type(item).__name__}"
        )

    try:
        label = _get_str(item, "label")
        mac_address = MacAddress.parse(_get_str(item, "mac_address"))
        host = _get_str(item, "host")
        target_address = SocketAddress.parse(_get_str(item, "target_address"))
    except ValueError as error:
        raise ValueError(f"Invalid item at position {n}: {error}") from error

    return Device(
        label=label,
        host=host,
        mac_address=mac_address,
        target_address=target_address,
    )


def load_devices(path: PathLike[str] | PathLike[bytes]) -> list[Device]:
    """Read devices from `path`."""
    with open(path, encoding="utf-8") as source:
        # Unfortunately, Python doesn't type Json for us, so we've got to cast here
        data = cast(Json, json.load(source))

    if not isinstance(data, list):
        raise ValueError(f"Expected list, got type {type(data).__name__}")

    return [_get_device(n, item) for n, item in enumerate(data)]


def dump_devices(path: PathLike[str] | PathLike[bytes], devices: list[Device]) -> None:
    """Dump `devices` to `path`, as JSON."""
    with open(path, "w", encoding="utf-8") as sink:
        json.dump(
            [
                {
                    "label": d.label,
                    "host": d.host,
                    "mac_address": str(d.mac_address),
                    "target_address": str(d.target_address),
                }
                for d in devices
            ],
            sink,
            indent=2,
        )


class DeviceStorage(Thread):
    """Automatically save devices."""

    def __init__(self, path: Path) -> None:
        """Create a new device storage thread."""
        super().__init__(name="save-devices-automatically", daemon=False)
        self._path = path
        self._save_condition = Condition()
        self._devices_to_save: list[Device] | None = None
        self._stop = False

    @override
    def run(self) -> None:
        while True:
            devices = None

            # Critical section: wait until we're notified, and then check whether
            # we've got to stop or have to save devices, but don't actually do any I/O
            # here.
            with self._save_condition:
                self._save_condition.wait()
                if self._stop:
                    return
                devices = self._devices_to_save
                self._devices_to_save = None

            if devices:
                log.message(f"Saving {len(devices)} device(s) to {self._path}")
                # Save devices but outside of our critical section.
                dump_devices(self._path, devices)

    def request_stop(self) -> None:
        """Request that this thread stops."""
        with self._save_condition:
            self._stop = True
            self._save_condition.notify_all()

    def request_save_devices(self, devices: list[Device]) -> None:
        """Request that this thread should save some devices."""
        with self._save_condition:
            self._devices_to_save = devices
            self._save_condition.notify_all()

    def _devices_changed(
        self,
        devices: Gio.ListModel[DeviceObject],
        position: int,
        removed: int,
        added: int,
    ) -> None:
        if 0 < added:
            # Monitor all new devices for changes
            for i in range(position, devices.get_n_items()):
                device = devices.get_item(i)
                assert device
                device.connect(
                    "notify",
                    lambda *args: self.request_save_devices(
                        [d.device for d in devices]
                    ),
                )

        # Save devices
        self.request_save_devices([d.device for d in devices])

    def save_automatically(self, devices: Gio.ListModel[DeviceObject]) -> None:
        """Monitor `devices` and save automatically on changes."""
        devices.connect("items-changed", self._devices_changed)
        for device in devices:
            device.connect(
                "notify",
                lambda *args: self.request_save_devices([d.device for d in devices]),
            )
