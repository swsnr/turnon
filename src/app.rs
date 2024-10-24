// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::{Object, Variant};
use gtk::gio::{ActionEntry, ApplicationFlags, DBusMethodInvocation};

use crate::{
    config::APP_ID, searchprovider::SEARCH_PROVIDER_2_IFACE_NAME, widgets::EditDeviceDialog,
};

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

    fn handle_search_provider(
        &self,
        sender: &str,
        object_path: &str,
        interface_name: &str,
        method_name: &str,
        _parameters: Variant,
        invocation: DBusMethodInvocation,
    ) {
        log::debug!(
            "Sender {sender} called method {method_name} of {interface_name} on object {object_path}"
        );
        assert!(interface_name == SEARCH_PROVIDER_2_IFACE_NAME);
        invocation.return_error(gtk::gio::IOErrorEnum::NotSupported, "Call not supported");
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
    use std::cell::RefCell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gettextrs::pgettext;
    use glib::{OptionArg, OptionFlags};
    use gtk::gio::RegistrationId;

    use crate::model::{Device, Devices};
    use crate::searchprovider::search_provider_2_interface;
    use crate::storage::{StorageService, StorageServiceClient};
    use crate::widgets::TurnOnApplicationWindow;

    #[derive(Default)]
    pub struct TurnOnApplication {
        model: Devices,
        registered_search_provider: RefCell<Option<RegistrationId>>,
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
            app.add_main_option(
                "turn-on-device",
                0.into(),
                OptionFlags::NONE,
                OptionArg::String,
                &pgettext(
                    "option.turn-on-device.description",
                    "Turn on a device by its label",
                ),
                Some(&pgettext("option.turn-on-device.arg.description", "LABEL")),
            )
        }
    }

    impl ApplicationImpl for TurnOnApplication {
        /// Start the application.
        ///
        /// Set the default icon name for all Gtk windows, and setup all actions
        /// of the application.
        ///
        /// Load all persisted devices into the application model, and arrange
        /// for the model to be persisted automatically if the device model
        /// changes.
        ///
        /// Register a search provider for the application.
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

            log::info!("Registering search provider");
            if let Some(connection) = app.dbus_connection() {
                let interface_info = search_provider_2_interface();
                let registration_id = connection
                    .register_object("/de/swsnr/turnon", &interface_info)
                    .method_call(glib::clone!(
                        #[strong]
                        app,
                        move |_connection,
                              sender,
                              object_path,
                              interface_name,
                              method_name,
                              parameters,
                              invocation| {
                            app.handle_search_provider(
                                sender,
                                object_path,
                                interface_name,
                                method_name,
                                parameters,
                                invocation,
                            );
                        }
                    ))
                    .build()
                    .unwrap();
                self.registered_search_provider
                    .replace(Some(registration_id));
            }
        }

        /// Activate the application.
        ///
        /// Presents the current active window of the application, or creates a
        /// new application window and presents it, if the application doesn't
        /// have an active window currently.
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
            let options = command_line.options_dict();
            if let Ok(Some(true)) = options.lookup("add-device") {
                log::debug!(
                    "Activating app.add-device action in response to command line argument"
                );
                // Activate application to show main window first
                self.obj().activate();
                self.obj().activate_action("add-device", None);
                glib::ExitCode::SUCCESS
            } else if let Ok(Some(label)) = options.lookup::<String>("turn-on-device") {
                log::debug!("Turning on device in response to command line argument");
                match self.model.find_device_by_label(&label) {
                    Some(device) => {
                        glib::spawn_future_local(glib::clone!(
                            #[strong]
                            command_line,
                            async move {
                                match device.wol().await {
                                    Ok(_) => {
                                        command_line
                                            .set_exit_status(glib::ExitCode::SUCCESS.value());
                                        command_line.print_literal(
                                            &pgettext(
                                                "option.turn-on-device.message",
                                                "Sent magic packet to mac address %1 of device %2\n",
                                            )
                                            .replace("%1", &device.mac_addr6().to_string())
                                            .replace("%2", &label),
                                        );
                                    }
                                    Err(error) => {
                                        command_line.printerr_literal(
                                            &pgettext(
                                                "option.turn-on-device.error",
                                                "Failed to turn on device %1: %2\n",
                                            )
                                            .replace("%1", &label)
                                            .replace("%2", &error.to_string()),
                                        );
                                        command_line
                                            .set_exit_status(glib::ExitCode::FAILURE.value());
                                    }
                                }
                                command_line.done();
                            }
                        ));
                        glib::ExitCode::SUCCESS
                    }
                    None => {
                        command_line.printerr_literal(
                            &pgettext(
                                "option.turn-on-device.error",
                                "No device found for label %s\n",
                            )
                            .replace("%s", &label),
                        );
                        glib::ExitCode::FAILURE
                    }
                }
            } else {
                self.obj().activate();
                glib::ExitCode::SUCCESS
            }
        }

        /// Shutdown the application.
        ///
        /// Deregister the search provider interface.
        fn shutdown(&self) {
            self.parent_shutdown();
            if let Some(registration_id) = self.registered_search_provider.replace(None) {
                if let Some(connection) = self.obj().dbus_connection() {
                    connection.unregister_object(registration_id).ok();
                }
            }
        }
    }

    impl GtkApplicationImpl for TurnOnApplication {}

    impl AdwApplicationImpl for TurnOnApplication {}
}
