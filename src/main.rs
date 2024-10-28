// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(warnings, clippy::all)]

use adw::prelude::*;
use app::TurnOnApplication;
use glib::gstr;
use gtk::gio;
use gtk::glib;

mod app;
mod config;
mod dbus;
mod gettext;

mod model;
mod net;
mod searchprovider;
mod storage;
mod widgets;

/// Set up logging.
///
/// If the process is connected to journald log structured events directly to journald.
///
/// Otherwise log to console.
///
/// `$TURNON_LOG` and `$TURNON_LOG_STYLE` configure log level and log style (for console logging)
fn setup_logging() {
    let env_var = "TURNON_LOG";
    let default_level = log::LevelFilter::Info;
    if systemd_journal_logger::connected_to_journal() {
        let logger = systemd_journal_logger::JournalLog::new()
            .unwrap()
            .with_extra_fields([("VERSION", env!("CARGO_PKG_VERSION"))]);
        let filter = env_filter::Builder::from_env(env_var)
            .filter_level(default_level)
            .build();
        let max_level = filter.filter();
        log::set_boxed_logger(Box::new(env_filter::FilteredLog::new(logger, filter))).unwrap();
        log::set_max_level(max_level);
    } else {
        env_logger::Builder::new()
            .filter_level(default_level)
            .parse_env(env_var)
            .init();
    }
    glib::log_set_default_handler(glib::rust_log_handler);
}

fn main() -> glib::ExitCode {
    setup_logging();

    use gettext::*;
    let locale_dir = config::locale_directory();
    log::debug!(
        "Initializing gettext with locale directory {}",
        locale_dir.display()
    );
    bindtextdomain(config::APP_ID, locale_dir).unwrap();
    textdomain(config::APP_ID).unwrap();
    bind_textdomain_codeset(config::APP_ID, gstr!("UTF-8")).unwrap();
    setlocale(LC_ALL, gstr!("")).unwrap();

    gio::resources_register_include!("turnon.gresource").unwrap();
    glib::set_application_name("Turn On");

    let app = TurnOnApplication::default();
    app.set_version(env!("CARGO_PKG_VERSION"));
    app.run()
}
