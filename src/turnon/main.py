# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Entry point for Turn On."""

import gettext
import locale
import os
import sys
from gettext import pgettext as C_
from pathlib import Path
from typing import Never

import gi
import gi.events
from packaging.version import Version

import turnon

gi.disable_legacy_autoinit()
gi.require_version("Adw", "1")


def main() -> Never:
    """Start the application, as main entry point.

    Setup environment and start the application.
    """
    from gi.repository import Gio, GLib

    from . import log

    app_id: str
    locale_dir: Path
    if turnon.is_installed_editable():
        log.message("Editable installation, setting up resource overlays")
        repo_dir = Path(__file__).parents[2]
        dist_dir = repo_dir / "dist"
        # In editable mode, ddd resource overlays to point to the resource files
        # so that we can edit the resource files without having to recompile the
        # resources file.
        resources_dir = repo_dir / "resources"
        generated_resources_dir = dist_dir / "resources"
        overlays = [
            os.environ.get("G_RESOURCE_OVERLAYS"),
            f"/de/swsnr/turnon={resources_dir}",
            f"/de/swsnr/turnon={generated_resources_dir}",
        ]
        os.environ["G_RESOURCE_OVERLAYS"] = os.pathsep.join(filter(None, overlays))
        # In editable mode, point GSettings to the compiled schemas in our build
        # directory
        schemas_dir = dist_dir / "schemas"
        dirs = [os.environ.get("GSETTINGS_SCHEMA_DIR"), str(schemas_dir)]
        os.environ["GSETTINGS_SCHEMA_DIR"] = os.pathsep.join(filter(None, dirs))
        # In editable mode, always use a .Devel app ID regardless of version, and
        # load translations from the build directory
        app_id = "de.swsnr.turnon.Devel"
        locale_dir = dist_dir / "locale"
    else:
        import importlib.resources

        # Read compiled resources
        with importlib.resources.as_file(
            turnon.resource_files() / "resources.gresource"
        ) as resource:
            log.info(f"Loading compiled resources from {resource}")
            Gio.resources_register(Gio.Resource.load(str(resource)))
        version = Version(turnon.version())
        app_id = "de.swsnr.turnon.Devel" if version.is_devrelease else "de.swsnr.turnon"
        prefix = Path("/app") if Path("/.flatpak-info").exists() else Path(sys.prefix)
        locale_dir = prefix / "share" / "locale"

    log.info(f"Loading translations from {locale_dir}")

    locale.setlocale(locale.LC_ALL, "")
    # Setup text domain for the C standard library, and by implication for glib,
    # which exposes translations to Gtk Builder and thus to blueprint.
    locale.bindtextdomain(app_id, locale_dir)
    locale.bind_textdomain_codeset(app_id, "UTF-8")
    locale.textdomain(app_id)
    # Setup text domain for Python's gettext, so that our messages in Python code
    # get translated.
    gettext.bindtextdomain(app_id, locale_dir)
    gettext.textdomain(app_id)

    GLib.set_application_name(C_("application-name", "Turn On"))

    # Import app only after we've set up resource overrides, etc. to make
    # sure that templates, translations, etc. are in place.
    from .app import TurnOnApplication

    app = TurnOnApplication(application_id=app_id)
    app.set_version(turnon.version())

    # Use GLib event policy as a context manager instead of setting the policy
    # explicitly as recommended in https://pygobject.gnome.org/guide/asynchronous.html
    #
    # This maintains forward compatibility with https://gitlab.gnome.org/GNOME/pygobject/-/merge_requests/503
    # in PyGObject 3.56, and the upcoming deprecation of event loop policies in
    # Python 3.14
    #
    # We also have to ignore typing here because for some reason pyright doesn't find
    # the gi.events module.
    with gi.events.GLibEventLoopPolicy():
        sys.exit(app.run(sys.argv))
