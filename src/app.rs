// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::gio::{ActionEntry, ApplicationFlags};

use crate::{config::APP_ID, widgets::EditDeviceDialog};

glib::wrapper! {
    pub struct TurnOnApplication(ObjectSubclass<imp::TurnOnApplication>)
        @extends adw::Application, gtk::Application, gtk::gio::Application,
        @implements gtk::gio::ActionGroup, gtk::gio::ActionMap;
}

impl TurnOnApplication {
    fn setup_actions(&self) {
        let actions = [
            ActionEntry::builder("add-device")
                .activate(|app: &TurnOnApplication, _, _| {
                    let dialog = EditDeviceDialog::new();
                    let devices = app.imp().model().clone();
                    dialog.connect_saved(move |_, device| {
                        log::debug!("Adding new device: {:?}", device.imp());
                        devices.add_device(device);
                    });
                    dialog.present(app.active_window().as_ref());
                })
                .build(),
            ActionEntry::builder("quit")
                .activate(|app: &TurnOnApplication, _, _| app.quit())
                .build(),
            ActionEntry::builder("about")
                .activate(|app: &TurnOnApplication, _, _| {
                    adw::AboutDialog::from_appdata(
                        "/de/swsnr/turnon/de.swsnr.turnon.metainfo.xml",
                        Some(env!("CARGO_PKG_VERSION")),
                    )
                    .present(app.active_window().as_ref());
                })
                .build(),
        ];
        self.add_action_entries(actions);

        self.set_accels_for_action("win.add-device", &["<Control>n"]);
        self.set_accels_for_action("window.close", &["<Control>w"]);
        self.set_accels_for_action("app.quit", &["<Control>q"]);
    }
}

impl Default for TurnOnApplication {
    fn default() -> Self {
        Object::builder()
            .property("application-id", APP_ID)
            .property("resource-base-path", "/de/swsnr/turnon")
            .property("flags", ApplicationFlags::HANDLES_COMMAND_LINE)
            .build()
    }
}

mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gettextrs::pgettext;
    use glib::{OptionArg, OptionFlags};

    use crate::model::{Device, Devices};
    use crate::storage::{StorageService, StorageServiceClient};
    use crate::widgets::TurnOnApplicationWindow;

    #[derive(Default)]
    pub struct TurnOnApplication {
        model: Devices,
    }

    impl TurnOnApplication {
        pub fn model(&self) -> &Devices {
            &self.model
        }

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

    impl ObjectImpl for TurnOnApplication {
        fn constructed(&self) {
            self.parent_constructed();

            let app = self.obj();
            app.add_main_option(
                "add-device",
                0.into(),
                OptionFlags::NONE,
                OptionArg::None,
                &pgettext("option.add-device.description", "Add a new device"),
                None,
            );
        }
    }

    impl ApplicationImpl for TurnOnApplication {
        fn startup(&self) {
            self.parent_startup();
            let app = self.obj();
            log::debug!("Application starting");
            gtk::Window::set_default_icon_name(super::APP_ID);

            app.setup_actions();

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
            log::debug!("Activating application");
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

        fn command_line(&self, command_line: &gtk::gio::ApplicationCommandLine) -> glib::ExitCode {
            log::debug!(
                "Handling command line. Remote? {}",
                command_line.is_remote()
            );
            self.obj().activate();
            let options = command_line.options_dict();
            if let Ok(Some(true)) = options.lookup("add-device") {
                log::debug!(
                    "Activating app.add-device action in response to command line argument"
                );
                self.obj().activate_action("add-device", None);
            }
            glib::ExitCode::SUCCESS
        }
    }

    impl GtkApplicationImpl for TurnOnApplication {}

    impl AdwApplicationImpl for TurnOnApplication {}
}
