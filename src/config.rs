// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::{GStr, gstr};
use gtk::gio::{self, resources_register};

/// The app ID to use.
pub const APP_ID: &GStr = gstr!("de.swsnr.turnon");

/// The Cargo package verson.
///
/// This provides the full version from `Cargo.toml`.
pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get [`CARGO_PKG_VERSION`] parsed.
fn cargo_pkg_version() -> semver::Version {
    semver::Version::parse(CARGO_PKG_VERSION).unwrap()
}

/// The version to use for release notes.
///
/// Returns [`CARGO_PKG_VERSION`] but with patch set to 0, and all pre and
/// build parts emptied.
///
/// This follows our versioning policy which uses patch releases only for
/// translation updates.
pub fn release_notes_version() -> semver::Version {
    let mut version = cargo_pkg_version();
    version.patch = 0;
    version.pre = semver::Prerelease::EMPTY;
    version.build = semver::BuildMetadata::EMPTY;
    version
}

pub const G_LOG_DOMAIN: &str = "TurnOn";

/// Whether the app is running in flatpak.
pub fn running_in_flatpak() -> bool {
    std::fs::exists("/.flatpak-info").unwrap_or_default()
}

/// Whether this is a development/nightly build.
pub fn is_development() -> bool {
    APP_ID.ends_with(".Devel")
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
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/build/schemas");
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
pub fn locale_directory() -> &'static GStr {
    if running_in_flatpak() {
        gstr!("/app/share/locale")
    } else {
        gstr!("/usr/share/locale")
    }
}

/// Load and register resource files from manifest directory in a debug build.
#[cfg(debug_assertions)]
pub fn register_resources() {
    // In a debug build load resources from a file
    let files = [
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/",
            "build/resources/resources.generated.gresource"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/",
            "build/resources/resources.data.gresource"
        ),
    ];
    for file in files {
        let resource =
            gio::Resource::load(file).expect("Fail to load resource, run 'just compile'!");
        resources_register(&resource);
    }
}

/// Register embedded resource data in a release build.
#[cfg(not(debug_assertions))]
pub fn register_resources() {
    let generated = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/",
        "build/resources/resources.generated.gresource"
    ));
    let data = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/",
        "build/resources/resources.data.gresource"
    ));
    for resource in [generated.as_slice(), data.as_slice()] {
        let bytes = glib::Bytes::from_static(resource);
        let resource = gio::Resource::from_data(&bytes).unwrap();
        resources_register(&resource);
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn release_notes_version_only_has_major_and_minor() {
        let version = super::release_notes_version();
        assert_eq!(version.major, super::cargo_pkg_version().major);
        assert_eq!(version.minor, super::cargo_pkg_version().minor);
        assert_eq!(version.patch, 0);
        assert!(version.pre.is_empty());
        assert!(version.build.is_empty());
    }
    #[test]
    fn release_notes_for_release_notes_version() {
        let metadata = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/de.swsnr.turnon.metainfo.xml"
        ))
        .unwrap();
        assert!(metadata.contains(&format!(
            "<release version=\"{}\"",
            super::release_notes_version()
        )));
    }

    #[test]
    fn no_release_notes_for_cargo_pkg_version() {
        let version = super::cargo_pkg_version();
        if version != super::release_notes_version() {
            let metadata = std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/de.swsnr.turnon.metainfo.xml"
            ))
            .unwrap();
            assert!(!metadata.contains(&format!("version=\"{version}\"")));
        }
    }
}
