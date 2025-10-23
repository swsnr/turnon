// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

#![deny(warnings, clippy::all, clippy::pedantic,
    // Do cfg(test) right
    clippy::cfg_not_test,
    clippy::tests_outside_test_module,
    // Guard against left-over debugging output
    clippy::dbg_macro,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::unimplemented,
    clippy::use_debug,
    clippy::todo,
    // Require correct safety docs
    clippy::undocumented_unsafe_blocks,
    clippy::unnecessary_safety_comment,
    clippy::unnecessary_safety_doc,
    // We must use Gtk's APIs to exit the app.
    clippy::exit,
    // Don't panic carelessly
    clippy::get_unwrap,
    clippy::unused_result_ok,
    clippy::unwrap_in_result,
    clippy::indexing_slicing,
    // Do not carelessly ignore errors
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    // Code smells
    clippy::float_cmp_const,
    clippy::string_to_string,
    clippy::if_then_some_else_none,
    clippy::large_include_file,
    // Disable as casts
    clippy::as_conversions,
)]
#![allow(clippy::enum_glob_use, clippy::module_name_repetitions)]

use std::ffi::CString;

use adw::prelude::*;
use app::TurnOnApplication;
use gnome_app_utils::i18n::gettext;
use gtk::glib;

mod app;
mod config;
mod dbus;
mod futures;
mod net;

use config::G_LOG_DOMAIN;

fn main() -> glib::ExitCode {
    gnome_app_utils::log::log_to_glib();

    let locale_dir = config::locale_directory();
    glib::debug!("Initializing gettext with locale directory {}", locale_dir);
    if let Err(error) = gettext::init_gettext(
        &CString::new(config::APP_ID).unwrap(),
        locale_dir.to_cstr().unwrap(),
    ) {
        glib::warn!("Failed to initialize gettext: {error}");
    }

    config::register_resources();
    glib::set_application_name("Turn On");

    let app = TurnOnApplication::default();
    app.set_version(config::CARGO_PKG_VERSION);
    app.run()
}
