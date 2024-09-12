// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use gtk::glib;

static APP_ID: &str = include_str!("./appid");

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Wakeup")
        .build();
    window.present();
}

fn main() -> glib::ExitCode {
    // Setup logging?  Use log crate and have it log to glibs logging?

    let app = adw::Application::builder()
        .application_id(APP_ID.trim())
        .build();

    app.connect_activate(build_ui);

    app.run()
}
