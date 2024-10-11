// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(warnings, clippy::all)]

use adw::prelude::*;
use gtk::gio;
use gtk::gio::SimpleAction;
use gtk::glib::{self, Variant};
use model::{Device, Devices};
use services::{StorageService, StorageServiceClient};
use widgets::WakeUpApplicationWindow;

mod log;
mod model;
mod ping;
mod services;
mod widgets;

use log::G_LOG_DOMAIN;

static APP_ID: &str = "de.swsnr.wakeup";

fn activate_about_action(app: &adw::Application, _action: &SimpleAction, _param: Option<&Variant>) {
    adw::AboutDialog::from_appdata(
        "/de/swsnr/wakeup/de.swsnr.wakeup.metainfo.xml",
        Some(env!("CARGO_PKG_VERSION")),
    )
    .present(app.active_window().as_ref());
}

fn save_automatically(model: &Devices, storage: StorageServiceClient) {
    model.connect_items_changed(move |model, pos, n_added, _| {
        glib::debug!("Device list changed, saving devices");
        storage.request_save_devices(model.into());
        // Persist devices whenever one device changes
        for n in pos..n_added {
            model.item(n).unwrap().connect_notify_local(
                None,
                glib::clone!(
                    #[strong]
                    storage,
                    #[weak]
                    model,
                    move |_, _| {
                        glib::debug!("One device was changed, saving devices");
                        storage.request_save_devices((&model).into());
                    }
                ),
            );
        }
    });
}

/// Handle application startup.
///
/// Create application actions.
fn startup_application(app: &adw::Application, model: &Devices) {
    glib::debug!("Application starting");
    gtk::Window::set_default_icon_name(APP_ID);

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

    glib::debug!("Initializing storage");
    let data_dir = glib::user_data_dir().join(APP_ID);
    let storage = StorageService::new(data_dir.join("devices.json"));

    glib::info!("Loading devices synchronously");
    let devices = match storage.load_sync() {
        Err(error) => {
            glib::error!(
                "Failed to load devices from {}: {}",
                storage.target().display(),
                error
            );
            Vec::new()
        }
        Ok(devices) => devices.into_iter().map(Device::from).collect(),
    };
    model.reset_devices(devices);
    save_automatically(model, storage.client());
    glib::spawn_future_local(storage.spawn());
}

fn activate_application(app: &adw::Application, model: &Devices) {
    match app.active_window() {
        Some(window) => {
            glib::debug!("Representing existing application window");
            window.present()
        }
        None => {
            glib::debug!("Creating new application window");
            WakeUpApplicationWindow::new(app, model).present();
        }
    }
}

fn main() -> glib::ExitCode {
    static GLIB_LOGGER: glib::GlibLogger = glib::GlibLogger::new(
        glib::GlibLoggerFormat::Structured,
        glib::GlibLoggerDomain::CrateTarget,
    );
    log::set_logger(&GLIB_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    gio::resources_register_include!("wakeup.gresource").unwrap();
    glib::set_application_name("WakeUp");

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
