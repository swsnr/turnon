# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Dialog to edit devices."""

import ipaddress

from gi.repository import Adw, GLib, GObject, Gtk

from ..model import Device, PureDevice
from ..net import MacAddress, SocketAddress
from .util import add_shortcuts


@Gtk.Template.from_resource("/de/swsnr/turnon/validation-indicator.ui")
class ValidationIndicator(Adw.Bin):
    """Indicator for valid or invalid input."""

    __gtype_name__ = "TurnOnValidationIndicator"

    indicator: Gtk.Stack = Gtk.Template.Child()
    valid: Gtk.Widget = Gtk.Template.Child()
    invalid: Gtk.Widget = Gtk.Template.Child()

    _is_valid = False
    _feedback = ""

    def __init__(self) -> None:
        """Create a new indicator."""
        super().__init__()

    @GObject.Property(type=bool, default=False)
    def is_valid(self) -> bool:
        """Whether the input is valid or not."""
        return self._is_valid

    @is_valid.setter
    def set_is_valid(self, is_valid: bool) -> None:
        """Set the validation state."""
        self._is_valid = is_valid
        child = self.valid if is_valid else self.invalid
        self.indicator.set_visible_child(child)

    @GObject.Property(type=str)
    def feedback(self) -> str:
        """Get the feedback text to show for invalid inputs."""
        return self._feedback

    @feedback.setter
    def set_feedback(self, feedback: str) -> None:
        """Set the feedback text."""
        self._feedback = feedback


@Gtk.Template.from_resource("/de/swsnr/turnon/edit-device-dialog.ui")
class EditDeviceDialog(Adw.Dialog):
    """A dialog to edit devices."""

    __gtype_name__ = "TurnOnEditDeviceDialog"

    _label = ""
    _mac_address = ""
    _host = ""
    _target_address = ""

    def __init__(self, device: Device | None = None) -> None:
        """Create a dialog.

        If a device is given, initialize fields from the device, and update the
        properties of the device when saved.
        """
        super().__init__()
        self._device = device

        if self._device is not None:
            dev = self._device.device
            # Initialize properties from the device
            self.label = dev.label
            self.mac_address = str(dev.mac_address)
            self.host = dev.host
            self.target_address = str(dev.target_address)
        else:
            # Pre-fill a reasonable default target address
            self.target_address = "255.255.255.255:9"

        # Update validation status for initial values
        self._validate()

    def _validate(self) -> None:
        """Validate all inputs."""
        is_valid = (
            self.label_valid
            and self.mac_address_valid
            and "invalid" not in self.host_indicator
            and "invalid" not in self.target_address_indicator
        )
        self.action_set_enabled("save", is_valid)

    @GObject.Signal(arg_types=[Device])  # pyright: ignore[reportUntypedFunctionDecorator]
    def saved(self, device: Device) -> None:
        """Signal emitted when the given device was saved."""
        pass

    @property
    def device(self) -> Device | None:
        """Get the device being edited.

        Not a GObject property.
        """
        return self._device

    @GObject.Property(type=str, default="")
    def label(self) -> str:
        """Get the device label."""
        return self._label

    @label.setter
    def set_label(self, value: str) -> None:
        """Set the label."""
        self._label = value
        self.notify("label-valid")
        self._validate()

    @GObject.Property(type=bool, default=False)
    def label_valid(self) -> bool:
        """Whether the label is valid."""
        return bool(self._label)

    @GObject.Property(type=str)
    def mac_address(self) -> str:
        """Get the MAC address."""
        return self._mac_address

    @mac_address.setter
    def set_mac_address(self, value: str) -> None:
        """Set the MAC address."""
        self._mac_address = value
        self.notify("mac-address-valid")
        self._validate()

    @GObject.Property(type=bool, default=False)
    def mac_address_valid(self) -> bool:
        """Whether the MAC address is valid."""
        return MacAddress.is_mac_address(self._mac_address)

    @GObject.Property(type=str)
    def host(self) -> str:
        """Get the device host."""
        return self._host

    @host.setter
    def set_host(self, value: str) -> None:
        """Set the device host."""
        self._host = value
        self.notify("host-indicator")
        self._validate()

    @GObject.Property(type=str)
    def host_indicator(self) -> str:
        """Get the type and validation indicator for the device host."""
        if not self._host:
            return "invalid-empty"
        try:
            ip = ipaddress.ip_address(self._host)
            if isinstance(ip, ipaddress.IPv4Address):
                return "ipv4"
            else:
                return "ipv6"
        except ValueError:
            # Check whether the user specified a port, and if so,
            # reject the input.
            #
            # See https://codeberg.org/swsnr/turnon/issues/40
            _, sep, port = self._host.rpartition(":")
            if sep and port.isdigit():
                return "invalid-socket-address"
            else:
                return "host"

    @GObject.Property(type=str)
    def target_address(self) -> str:
        """Get the target address of the device."""
        return self._target_address

    @target_address.setter
    def set_target_address(self, value: str) -> None:
        """Set the target address of the device."""
        self._target_address = value
        self.notify("target-address-indicator")
        self._validate()

    @GObject.Property(type=str, default="invalid")
    def target_address_indicator(self) -> str:
        """Get the validation and type indicator for the target address."""
        try:
            sockaddr = SocketAddress.parse(self._target_address)
            if isinstance(sockaddr.address, ipaddress.IPv4Address):
                return "ipv4"
            else:
                return "ipv6"
        except ValueError:
            return "invalid"

    @Gtk.Template.Callback()
    @staticmethod
    def move_to_next_entry(entry: Adw.EntryRow) -> None:
        """Forward to the next entry from the given `entry`."""
        entry.emit("move-focus", Gtk.DirectionType.TAB_FORWARD)


def _activate_device_save(
    dialog: Gtk.Widget, _action_name: str, _param: GLib.Variant | None
) -> None:
    assert isinstance(dialog, EditDeviceDialog)
    device = dialog.device
    # We can trust all inputs, as the save action is only enabled if the dialog
    # inputs are valid
    mac_address = MacAddress.parse(dialog.mac_address)
    target_address = SocketAddress.parse(dialog.target_address)
    if device:
        device.label = dialog.label
        device.mac_address = mac_address
        device.host = dialog.host
        device.target_address = target_address
    else:
        device = Device(
            PureDevice(
                label=dialog.label,
                mac_address=mac_address,
                host=dialog.host,
                target_address=target_address,
            )
        )
    dialog.emit("saved", device)
    dialog.close()


EditDeviceDialog.install_action("save", None, _activate_device_save)

add_shortcuts(EditDeviceDialog, [("<Ctrl>S", "action(save)")])
