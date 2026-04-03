# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Turn On.

Gnome desktop application to wake up devices in a local network with Wake On LAN.
"""

from importlib.resources.abc import Traversable


def version() -> str:
    """Get our version, from distribution metadata."""
    from importlib.metadata import version

    return version(__name__)


def is_installed_editable() -> bool:
    """Whether the application is installed in editable mode for development."""
    from importlib.metadata import distribution

    dist = distribution(__name__)

    if dist.origin and hasattr(dist.origin, "dir_info"):
        return getattr(dist.origin.dir_info, "editable", False)

    return False


def resource_files() -> Traversable:
    """Get our resource files."""
    from importlib.resources import files

    return files(__name__)


def license_text() -> str:
    """Get the full text of our license."""
    from importlib.metadata import files

    our_files = files(__name__) or []
    license_file = next((f for f in our_files if f.name == "LICENSE"), None)
    assert license_file is not None
    return license_file.read_text()
