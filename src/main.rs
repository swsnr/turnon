// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(warnings, clippy::all)]
#![forbid(unsafe_code)]

use std::rc::Rc;

use adw::prelude::*;
use gtk::gio;
use gtk::gio::SimpleAction;
use gtk::glib::{self, Variant};
use model::{Device, Devices};
use storage::{DevicesStorage, StoredDevice};
use widgets::WakeUpApplicationWindow;

mod model;
mod storage;
mod widgets;

static APP_ID: &str = "de.swsnr.wakeup";

fn write_devices_async(storage: Rc<DevicesStorage>, model: &Devices) -> glib::JoinHandle<()> {
    let stored_data: Vec<StoredDevice> = model.into();
    glib::spawn_future_local(glib::clone!(
        #[strong]
        storage,
        async move {
            println!("Saving devices to storage");
            if let Err(err) = storage.save(stored_data).await {
                // TODO: Log error properly!
                eprintln!("Failed to save devices: {:?}", err);
            }
        }
    ))
}

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
fn startup_application(app: &adw::Application, storage: Rc<DevicesStorage>, model: &Devices) {
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

    glib::spawn_future_local(glib::clone!(
        #[strong]
        model,
        async move {
            match storage.load().await {
                Ok(stored_devices) => {
                    let devices = stored_devices.into_iter().map(Device::from).collect();
                    model.reset_devices(devices);
                }
                Err(err) => {
                    // TODO: Log error properly
                    eprintln!("Failed to load devices: {:?}", err);
                }
            }

            // After we loaded the model, persist it automatically whenever it
            // changes.
            model.connect_items_changed(glib::clone!(
                #[strong]
                storage,
                move |model, pos, n_added, _| {
                    write_devices_async(storage.clone(), model);
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
                                    write_devices_async(storage.clone(), &model);
                                }
                            ),
                        );
                    }
                }
            ));
        }
    ));
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

    let data_dir = glib::user_data_dir().join(APP_ID);
    let storage = Rc::new(DevicesStorage::new(data_dir.join("devices.json")));
    let model = Devices::default();

    app.connect_activate(glib::clone!(
        #[strong]
        model,
        move |app| activate_application(app, &model)
    ));
    app.connect_startup(glib::clone!(
        #[strong]
        model,
        move |app| startup_application(app, storage.clone(), &model)
    ));

    app.run()
}
