# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Widgets for Turn On."""

from gi.repository import Adw, Gtk

from turnon.model import Device, Devices


@Gtk.Template.from_resource("/de/swsnr/turnon/turnon-application-window.ui")
class TurnOnApplicationWindow(Adw.ApplicationWindow):
    """Main application window for Turn On."""

    __gtype_name__: str = "TurnOnApplicationWindow"

    devices_list: Gtk.ListBox = Gtk.Template.Child()

    def __init__(self, application: Adw.Application, devices: Devices) -> None:
        """Create an application window for the given application."""
        super().__init__(application=application)
        self._devices = Devices
        self.devices_list.bind_model(devices, self._create_device_row)

    def _create_device_row(self, device: Device) -> Gtk.Widget:
        return Gtk.Label.new(device.label)
