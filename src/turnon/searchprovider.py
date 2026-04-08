# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""The search provider for Turn On."""

from dataclasses import asdict, dataclass
from functools import cache
from gettext import pgettext as C_
from typing import cast

from gi.repository import Gio, GLib, GObject

from .dbus import load_dbus_interface
from .model import DeviceObject
from .net import wol


def _check_type(variant: GLib.Variant, expected: str) -> None:
    ts = variant.get_type_string()
    if ts != expected:
        raise GLib.Error(
            domain=GLib.quark_to_string(Gio.DBusError.quark()),
            code=Gio.DBusError.INVALID_ARGS,
            message=f"Expected {expected} but got {ts}",
        )


@dataclass(kw_only=True)
class ResultMeta:
    """Result meta information in the search provider interface."""

    id: str
    name: str
    description: str

    def to_variant(self) -> dict[str, GLib.Variant]:
        """Convert to a variant dict."""
        return {k: GLib.Variant("s", v) for k, v in asdict(self).items()}


@cache
def search_provider_interface() -> Gio.DBusInterfaceInfo:
    """Get the search provider D-Bus interface description."""
    return load_dbus_interface("org.gnome.Shell.SearchProvider2")


def _matches(device: DeviceObject, terms: list[str]) -> bool:
    return all(
        t in device.device.label.lower() or t in device.device.host.lower()
        for t in terms
    )


class SearchProvider(GObject.Object):
    """Search provider for Turn On."""

    __gtype_name__ = "TurnOnSearchProvider"

    def __init__(self, id_prefix: str, devices: Gio.ListModel[DeviceObject]) -> None:
        """Create a search provider for `devices`."""
        super().__init__()
        self._id_prefix = id_prefix
        self._devices = devices

    def _get_initial_result_set(self, terms: list[str]) -> list[str]:
        return [
            f"{self._id_prefix}-{i}"
            for i, device in enumerate(self._devices)
            if _matches(device, terms)
        ]

    def _lookup_by_id(self, id: str) -> DeviceObject | None:
        return self._devices.get_item(int(id[len(self._id_prefix) + 1 :]))

    def _get_subsearch_results(
        self, prev_results: list[str], terms: list[str]
    ) -> list[str]:
        # We don't care for previous results, as our model is small enough
        return [
            id
            for id, device in ((id, self._lookup_by_id(id)) for id in prev_results)
            if device is not None and _matches(device, terms)
        ]

    def _get_result_metas(self, ids: list[str]) -> list[ResultMeta]:
        devices = ((id, self._lookup_by_id(id)) for id in ids)
        return [
            ResultMeta(id=id, name=d.device.label, description=d.device.host)
            for id, d in devices
            if d is not None
        ]

    async def _activate_result(self, id: str) -> None:
        device = self._lookup_by_id(id)
        if device is None:
            raise GLib.Error(
                domain=GLib.quark_to_string(Gio.DBusError.quark()),
                code=Gio.DBusError.INVALID_ARGS,
                message=f"Device with ID {id} not found",
            )
        try:
            await wol(device.device.mac_address, device.device.target_address)
            notification = Gio.Notification.new(
                C_(
                    "search-provider.notification.title",
                    "Sent magic packet",
                )
            )
            notification.set_body(
                C_(
                    "search-provider.notification.body",
                    "Sent magic packet to mac address {device_mac_address} "
                    + "of device {device_label}.",
                ).format(
                    device_mac_address=str(device.device.mac_address),
                    device_label=device.device.label,
                )
            )
            self.emit("send-notification", notification, 10)
        except BaseException:
            notification = Gio.Notification.new(
                C_(
                    "search-provider.notification.title",
                    "Failed to send magic packet",
                )
            )
            notification.set_body(
                C_(
                    "search-provider.notification.body",
                    "Failed to send magic packet to mac address "
                    + "{device_mac_address} of device {device_label}.",
                ).format(
                    device_mac_address=str(device.device.mac_address),
                    device_label=device.device.label,
                )
            )
            self.emit("send-notification", notification, -1)
            raise

    @GObject.Signal(arg_types=[object])  # pyright: ignore[reportUntypedFunctionDecorator]
    def search_launched(self, terms: list[str]) -> None:
        """Signal emitted when search is launched."""
        pass

    @GObject.Signal(arg_types=[Gio.Notification, int])  # pyright: ignore[reportUntypedFunctionDecorator]
    def send_notification(self, notification: Gio.Notification, timeout: int) -> None:
        """Signal emitted when a notification should be shown by the app."""
        pass

    async def call_method(
        self, method: str, params: GLib.Variant
    ) -> GLib.Variant | None:
        """Asynchronously call a method on this search provider."""
        match method:
            case "GetInitialResultSet":
                _check_type(params, "(as)")
                (terms,) = cast(tuple[list[str]], params.unpack())
                ids = self._get_initial_result_set(terms)
                return GLib.Variant("(as)", (ids,))
            case "GetSubsearchResultSet":
                _check_type(params, "(asas)")
                (prev_results, terms) = cast(
                    tuple[list[str], list[str]], params.unpack()
                )
                ids = self._get_subsearch_results(prev_results, terms)
                return GLib.Variant("(as)", (ids,))
            case "GetResultMetas":
                _check_type(params, "(as)")
                (ids,) = cast(tuple[list[str]], params.unpack())
                return GLib.Variant(
                    "(aa{sv})",
                    ([meta.to_variant() for meta in self._get_result_metas(ids)],),
                )
            case "ActivateResult":
                _check_type(params, "(sasu)")
                (id, _, _) = cast(tuple[str, list[str], int], params.unpack())
                await self._activate_result(id)
                return None
            case "LaunchSearch":
                _check_type(params, "(asu)")
                (terms, _) = cast(tuple[list[str], int], params.unpack())
                self.emit("search-launched", terms)
                return
            case other:
                raise GLib.Error(
                    domain=GLib.quark_to_string(Gio.DBusError.quark()),
                    code=Gio.DBusError.UNKNOWN_METHOD,
                    message=f"Unknown method: {other}",
                )
