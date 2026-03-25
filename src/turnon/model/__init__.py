# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""GObject based data model for Turn On.

The `pure` sub-package contains the pure-Python data model backing the GObject
types.
"""

from .gobject import Device, Devices

__all__ = ["Device", "Devices"]
