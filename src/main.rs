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

use config::G_LOG_DOMAIN;

fn main() -> glib::ExitCode {
    let max_level = if std::env::var_os("G_MESSAGES_DEBUG").is_some() {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Warn
    };
    static GLIB_LOGGER: glib::GlibLogger = glib::GlibLogger::new(
        glib::GlibLoggerFormat::Structured,
        glib::GlibLoggerDomain::CrateTarget,
    );
    log::set_max_level(max_level);
    log::set_logger(&GLIB_LOGGER).unwrap();

    use gettext::*;
    let locale_dir = config::locale_directory();
    glib::debug!(
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
