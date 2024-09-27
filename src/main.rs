// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(warnings, clippy::all)]
#![forbid(unsafe_code)]

use adw::prelude::*;
use gtk::gio;
use gtk::gio::SimpleAction;
use gtk::glib::{self, Variant};
use model::Devices;
use widgets::WakeUpApplicationWindow;

mod model;
mod widgets;

static APP_ID: &str = "de.swsnr.wakeup";

fn activate_about_action(app: &adw::Application, _action: &SimpleAction, _param: Option<&Variant>) {
    adw::AboutDialog::from_appdata(
        "/de/swsnr/wakeup/de.swsnr.wakeup.metainfo.xml",
        Some(env!("CARGO_PKG_VERSION")),
    )
    .present(app.active_window().as_ref());
}

/// Handle application startup.
///
/// Create application actions.
fn startup_application(app: &adw::Application, _model: &Devices) {
    let actions = [
        gio::ActionEntryBuilder::new("quit")
            .activate(|a: &adw::Application, _, _| a.quit())
            .build(),
        gio::ActionEntryBuilder::new("about")
            .activate(activate_about_action)
            .build(),
    ];
    app.add_action_entries(actions);

    app.set_accels_for_action("win.add_device", &["<Control>n"]);
    app.set_accels_for_action("window.close", &["<Control>w"]);
    app.set_accels_for_action("app.quit", &["<Control>q"]);

    // TODO: Load model here
}

fn activate_application(app: &adw::Application, model: &Devices) {
    match app.active_window() {
        Some(window) => window.present(),
        None => {
            WakeUpApplicationWindow::new(app, model).present();
        }
    }
}

fn main() -> glib::ExitCode {
    // Setup logging?  Use log crate and have it log to glibs logging?

    gio::resources_register_include!("wakeup.gresource").unwrap();
    glib::set_application_name("WakeUp");
    gtk::Window::set_default_icon_name(APP_ID);

    let app = adw::Application::builder()
        .application_id(APP_ID.trim())
        .build();

    let model = Devices::default();

    app.connect_activate(glib::clone!(
        #[strong]
        model,
        move |app| activate_application(app, &model)
    ));
    app.connect_startup(glib::clone!(
        #[strong]
        model,
        move |app| startup_application(app, &model)
    ));

    app.run()
}
