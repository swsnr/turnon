// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(warnings, clippy::all)]
#![forbid(unsafe_code)]

use adw::prelude::*;
use gtk::gio;
use gtk::glib;
use widgets::WakeUpApplicationWindow;

mod widgets;

static APP_ID: &str = "de.swsnr.wakeup";

fn build_ui(app: &adw::Application) {
    // TODO: Create mainwindow from window.ui file
    let window = WakeUpApplicationWindow::new(app);
    window.present();
}

fn main() -> glib::ExitCode {
    // Setup logging?  Use log crate and have it log to glibs logging?

    gio::resources_register_include!("wakeup.gresource").unwrap();

    let app = adw::Application::builder()
        .application_id(APP_ID.trim())
        .build();

    app.connect_activate(build_ui);

    app.run()
}
