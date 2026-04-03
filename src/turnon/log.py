# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Logging utilities."""

import asyncio
import traceback

from gi.repository import GLib

LOG_DOMAIN = "TurnOn"


def log(level: GLib.LogLevelFlags, message: str) -> None:
    """Log a message at a given level."""
    dict = GLib.VariantDict.new()
    dict.insert_value("MESSAGE", GLib.Variant.new_string(message))
    GLib.log_variant(LOG_DOMAIN, level, dict.end())


def warn(message: str) -> None:
    """Log a warning message."""
    log(GLib.LogLevelFlags.LEVEL_WARNING, message)


def message(message: str) -> None:
    """Log a normal message."""
    log(GLib.LogLevelFlags.LEVEL_MESSAGE, message)


def info(message: str) -> None:
    """Log an info message."""
    log(GLib.LogLevelFlags.LEVEL_INFO, message)


def debug(message: str) -> None:
    """Log an debug message."""
    log(GLib.LogLevelFlags.LEVEL_DEBUG, message)


def log_task_exception[T](task: asyncio.Task[T]) -> None:
    """Log exception of a failed task.

    For use as done callback for tasks.
    """
    if task.cancelled():
        return
    exception = task.exception()
    if exception is not None:
        message = "".join(traceback.format_exception(exception))
        warn(f"Task {task.get_name()} failed: {message}")
