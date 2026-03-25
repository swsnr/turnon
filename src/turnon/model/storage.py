# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12


"""Store and load lists of devices in JSON files."""

import json
from os import PathLike
from typing import cast

from turnon.net import MacAddress, SocketAddress

from .pure import Device

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
