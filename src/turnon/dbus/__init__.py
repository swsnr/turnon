# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Utilities for working with D-Bus."""

import asyncio
import importlib.resources
from collections.abc import Callable, Coroutine
from functools import partial
from typing import Any

from gi.repository import Gio, GLib

from .. import log


def dbus_error(code: int, message: str) -> GLib.Error:
    """Create a DBus error."""
    return GLib.Error(
        domain=GLib.quark_to_string(Gio.DBusError.quark()), message=message, code=code
    )


def load_dbus_interface(name: str) -> Gio.DBusInterfaceInfo:
    """Load the D-Bus interface with `name`."""
    iface = Gio.DBusNodeInfo.new_for_xml(
        (importlib.resources.files() / f"{name}.xml").read_text()
    ).lookup_interface(name)
    assert iface is not None
    return iface


def _finish_invocation(
    invocation: Gio.DBusMethodInvocation, task: asyncio.Task[GLib.Variant | None]
) -> None:
    try:
        invocation.return_value(task.result())
    except asyncio.CancelledError:
        invocation.return_error_literal(
            domain=Gio.DBusError.quark(),
            code=Gio.DBusError.NO_REPLY,
            message="Call cancelled",
        )
    except GLib.Error as error:
        invocation.return_gerror(error)
    except TimeoutError as error:
        invocation.return_error_literal(
            domain=Gio.DBusError.quark(),
            code=Gio.DBusError.TIMEOUT,
            message=str(error),
        )
    except BaseException as error:
        invocation.return_error_literal(
            domain=Gio.DBusError.quark(),
            code=Gio.DBusError.FAILED,
            message=str(error),
        )


class DBusObject:
    """A registered D-Bus object."""

    def __init__(
        self,
        objpath: str,
        interface: Gio.DBusInterfaceInfo,
        call_method: Callable[
            [str, GLib.Variant], Coroutine[Any, Any, GLib.Variant | None]
        ],
    ) -> None:
        """Create a D-Bus object.

        `objpath` is the path on which the object will be registered, and `interface`
        is the interface description it implements.

        `call_method` is the callback to dispatch method calls.
        """
        super().__init__()
        self._objpath = objpath
        self._interface = interface
        self._call_method = call_method
        self._ongoing_calls: set[asyncio.Task[GLib.Variant | None]] = set()

    def __call__(
        self,
        _connection: Gio.DBusConnection,
        _sender: str,
        objpath: str,
        interface_name: str,
        method_name: str,
        params: GLib.Variant,
        invocation: Gio.DBusMethodInvocation,
    ) -> None:
        """Call a method on this object."""
        if interface_name != self._interface.name:
            invocation.return_error_literal(
                domain=Gio.DBusError.quark(),
                code=Gio.DBusError.UNKNOWN_INTERFACE,
                message=f"Unknown interface {interface_name}",
            )
        elif objpath != self._objpath:
            invocation.return_error_literal(
                domain=Gio.DBusError.quark(),
                code=Gio.DBusError.UNKNOWN_OBJECT,
                message=f"Unknown object {objpath}",
            )
        else:
            task = asyncio.create_task(self._call_method(method_name, params))
            task.add_done_callback(self._ongoing_calls.discard)
            task.add_done_callback(log.log_task_exception)
            task.add_done_callback(partial(_finish_invocation, invocation))
            self._ongoing_calls.add(task)

    def register_on(self, connection: Gio.DBusConnection) -> int:
        """Register this object on `connection`.

        Return the registration ID for later unregistration.
        """
        return connection.register_object(self._objpath, self._interface, self)
