// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::PathBuf;

use glib::{gstr, GStr};
use gtk::gio;

/// The app ID to use.
pub static APP_ID: &GStr = gstr!("de.swsnr.turnon");

/// The app version.
pub static VERSION: &str = env!("CARGO_PKG_VERSION");

pub const G_LOG_DOMAIN: &str = "TurnOn";

/// Whether the app is running in flatpak.
pub fn running_in_flatpak() -> bool {
    std::fs::exists("/.flatpak-info").unwrap_or_default()
}

/// Get a schema source for this application.
///
/// In a debug build load compiled schemas from the manifest directory, to allow
/// running the application uninstalled.
///
/// In a release build only use the default schema source.
pub fn schema_source() -> gio::SettingsSchemaSource {
    let default = gio::SettingsSchemaSource::default().unwrap();
    if cfg!(debug_assertions) {
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/schemas");
        if std::fs::exists(directory).unwrap_or_default() {
            gio::SettingsSchemaSource::from_directory(directory, Some(&default), false).unwrap()
        } else {
            default
        }
    } else {
        default
    }
}

/// Get the locale directory.
///
/// Return the flatpak locale directory when in
pub fn locale_directory() -> PathBuf {
    if let Some(dir) = std::env::var_os("TURNON_LOCALE_DIR") {
        dir.into()
    } else if running_in_flatpak() {
        "/app/share/locale".into()
    } else {
        "/usr/share/locale".into()
    }
}
