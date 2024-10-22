// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use glib::{Object, Variant};
use gtk::gio::SimpleAction;

use crate::config::APP_ID;

glib::wrapper! {
    pub struct TurnOnApplication(ObjectSubclass<imp::TurnOnApplication>)
        @extends adw::Application, gtk::Application, gtk::gio::Application,
        @implements gtk::gio::ActionGroup, gtk::gio::ActionMap;
}

impl TurnOnApplication {
    fn activate_about_action(&self, _action: &SimpleAction, _param: Option<&Variant>) {
        adw::AboutDialog::from_appdata(
            "/de/swsnr/turnon/de.swsnr.turnon.metainfo.xml",
            Some(env!("CARGO_PKG_VERSION")),
        )
        .present(self.active_window().as_ref());
    }

    fn activate_quit(&self, _action: &SimpleAction, _param: Option<&Variant>) {
        self.quit();
    }
}

impl Default for TurnOnApplication {
    fn default() -> Self {
        Object::builder()
            .property("application-id", APP_ID)
            .property("resource-base-path", "/de/swsnr/turnon")
            .build()
    }
}

mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::gio;

    use crate::{
        model::{Device, Devices},
        services::{StorageService, StorageServiceClient},
        widgets::TurnOnApplicationWindow,
    };

    #[derive(Default)]
    pub struct TurnOnApplication {
        model: Devices,
    }

    impl TurnOnApplication {
        fn save_automatically(&self, storage: StorageServiceClient) {
            self.model
                .connect_items_changed(move |model, pos, _, n_added| {
                    log::debug!("Device list changed, saving devices");
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
                                    log::debug!("One device was changed, saving devices");
                                    storage.request_save_devices((&model).into());
                                }
                            ),
                        );
                    }
                });
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TurnOnApplication {
        const NAME: &'static str = "TurnOnApplication";

        type Type = super::TurnOnApplication;

        type ParentType = adw::Application;
    }

    impl ObjectImpl for TurnOnApplication {}

    impl ApplicationImpl for TurnOnApplication {
        fn startup(&self) {
            self.parent_startup();
            let app = self.obj();
            log::debug!("Application starting");
            gtk::Window::set_default_icon_name(super::APP_ID);

            let actions = [
                gio::ActionEntryBuilder::new("quit")
                    .activate(super::TurnOnApplication::activate_quit)
                    .build(),
                gio::ActionEntryBuilder::new("about")
                    .activate(super::TurnOnApplication::activate_about_action)
                    .build(),
            ];
            app.add_action_entries(actions);

            app.set_accels_for_action("win.add_device", &["<Control>n"]);
            app.set_accels_for_action("window.close", &["<Control>w"]);
            app.set_accels_for_action("app.quit", &["<Control>q"]);

            log::debug!("Initializing storage");
            let data_dir = glib::user_data_dir().join(super::APP_ID);
            let storage = StorageService::new(data_dir.join("devices.json"));

            log::info!("Loading devices synchronously");
            let devices = match storage.load_sync() {
                Err(error) => {
                    log::error!(
                        "Failed to load devices from {}: {}",
                        storage.target().display(),
                        error
                    );
                    Vec::new()
                }
                Ok(devices) => devices.into_iter().map(Device::from).collect(),
            };
            self.model.reset_devices(devices);
            self.save_automatically(storage.client());
            glib::spawn_future_local(storage.spawn());
        }

        fn activate(&self) {
            self.parent_activate();
            let app: &super::TurnOnApplication = &self.obj();
            match app.active_window() {
                Some(window) => {
                    log::debug!("Representing existing application window");
                    window.present()
                }
                None => {
                    log::debug!("Creating new application window");
                    TurnOnApplicationWindow::new(app, &self.model).present();
                }
            }
        }
    }

    impl GtkApplicationImpl for TurnOnApplication {}

    impl AdwApplicationImpl for TurnOnApplication {}
}
