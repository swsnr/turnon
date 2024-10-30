// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::PathBuf;

use glib::{gstr, GStr};

/// The app ID to use.
pub static APP_ID: &GStr = gstr!("de.swsnr.turnon");

pub const G_LOG_DOMAIN: &str = "TurnOn";

/// Whether the app is running in flatpak.
fn running_in_flatpak() -> bool {
    std::fs::exists("/.flatpak-info").unwrap_or_default()
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
