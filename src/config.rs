// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use formatx::formatx;
use glib::{GStr, dpgettext2, gstr};
use gnome_app_utils::env::running_in_flatpak;
use gtk::gio::{self, resources_register};

pub const APP_ID: &str = include_str!("../build/app-id");

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

/// The full app license text.
pub const LICENSE_TEXT: &str = include_str!("../LICENSE");

/// URL for official translations of the EUPL 1.2 text.
const LICENSE_TRANSLATIONS_URL: &str =
    "https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12";

pub fn license_text() -> String {
    formatx!(
        dpgettext2(
            None,
            "about-dialog.license-text",
            // Translators: This is Pango markup, be sure to escape appropriately
            "Copyright {copyright_name} &lt;{copyright_email}&gt;

Licensed under the terms of the EUPL 1.2. You can find official translations \
of the license text at <a href=\"{translations}\">{translations}</a>.

The full English text follows.

{license_text}",
        ),
        copyright_name = "Sebastian Wiesner",
        copyright_email = "sebastian@swsnr.de",
        translations = LICENSE_TRANSLATIONS_URL,
        license_text = glib::markup_escape_text(LICENSE_TEXT)
    )
    .unwrap()
}

/// Whether this is a development/nightly build.
pub fn is_development() -> bool {
    APP_ID.ends_with(".Devel")
}

/// Get schema source for a debug build.
///
/// Load schemas from `build/schemas` in the manifest directory, and fallback
/// to the default source.
#[cfg(debug_assertions)]
fn schema_source() -> Option<gio::SettingsSchemaSource> {
    let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/build/schemas");
    std::fs::exists(directory).unwrap_or_default().then(|| {
        let default = gio::SettingsSchemaSource::default().unwrap();
        gio::SettingsSchemaSource::from_directory(directory, Some(&default), false).unwrap()
    })
}

/// Get schema source in a release build.
///
/// Simply return `None` to use the default source.
#[cfg(not(debug_assertions))]
fn schema_source() -> Option<gio::SettingsSchemaSource> {
    None
}

/// Get settings for this application.
pub fn get_settings() -> gio::Settings {
    match schema_source() {
        Some(source) => gio::Settings::new_full(
            &source.lookup(crate::config::APP_ID, true).unwrap(),
            gio::SettingsBackend::NONE,
            None,
        ),
        None => gio::Settings::new(crate::config::APP_ID),
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
