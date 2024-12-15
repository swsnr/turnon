// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::{dgettext, dpgettext2, Object};
use gtk::gio::{ActionEntry, ApplicationFlags};

use crate::config::{APP_ID, G_LOG_DOMAIN};

mod commandline;
mod debuginfo;
mod model;
mod searchprovider;
mod storage;
mod widgets;

use debuginfo::DebugInfo;
use widgets::EditDeviceDialog;

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
                    let devices = app.devices().registered_devices();
                    dialog.connect_saved(move |_, device| {
                        glib::debug!("Adding new device: {:?}", device.imp());
                        devices.append(device);
                    });
                    dialog.present(app.active_window().as_ref());
                })
                .build(),
            ActionEntry::builder("quit")
                .activate(|app: &TurnOnApplication, _, _| app.quit())
                .build(),
            ActionEntry::builder("about")
                .activate(|app: &TurnOnApplication, _, _| {
                    let dialog = adw::AboutDialog::from_appdata(
                        "/de/swsnr/turnon/de.swsnr.turnon.metainfo.xml",
                        Some(crate::config::VERSION),
                    );

                    glib::spawn_future_local(glib::clone!(
                        #[strong(rename_to = devices)]
                        app.devices(),
                        #[weak]
                        dialog,
                        async move {
                            let info = DebugInfo::assemble(devices).await;
                            dialog.set_debug_info(&info.to_string());
                            dialog.set_debug_info_filename(&info.suggested_file_name());
                        }
                    ));

                    dialog.add_link(
                        &dpgettext2(None, "about-dialog.link.label", "Translations"),
                        "https://translate.codeberg.org/engage/de-swsnr-turnon/",
                    );

                    dialog.set_developers(&["Sebastian Wiesner https://swsnr.de"]);
                    dialog.set_designers(&["Sebastian Wiesner https://swsnr.de"]);
                    // Credits for the translator to the current language.
                    // Translators: Add your name here, as "Jane Doe <jdoe@example.com>" or "Jane Doe https://jdoe.example.com"
                    // Mail address or URL are optional.  Separate multiple translators with a newline, i.e. \n
                    dialog.set_translator_credits(&dgettext(None, "translator-credits"));
                    dialog.add_acknowledgement_section(
                        Some(&dpgettext2(
                            None,
                            "about-dialog.acknowledgment-section",
                            "Help and inspiration",
                        )),
                        &[
                            "Sebastian Dröge https://github.com/sdroege",
                            "Bilal Elmoussaoui https://github.com/bilelmoussaoui",
                            "Authenticator https://gitlab.gnome.org/World/Authenticator",
                            "Decoder https://gitlab.gnome.org/World/decoder/",
                        ],
                    );
                    dialog.add_acknowledgement_section(
                        Some(&dpgettext2(
                            None,
                            "about-dialog.acknowledgment-section",
                            "Helpful services",
                        )),
                        &[
                            "Flathub https://flathub.org/",
                            "Open Build Service https://build.opensuse.org/",
                            "GitHub actions https://github.com/features/actions",
                        ],
                    );

                    dialog.present(app.active_window().as_ref());
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
    use std::cell::RefCell;
    use std::path::PathBuf;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{dpgettext2, OptionArg, OptionFlags};
    use gtk::gio::{ListStore, RegistrationId};

    use crate::config::G_LOG_DOMAIN;

    use super::commandline;
    use super::model::{Device, Devices};
    use super::searchprovider::register_app_search_provider;
    use super::storage::{StorageService, StorageServiceClient};
    use super::widgets::TurnOnApplicationWindow;

    #[derive(glib::Properties)]
    #[properties(wrapper_type = super::TurnOnApplication)]
    pub struct TurnOnApplication {
        #[property(get)]
        devices: Devices,
        registered_search_provider: RefCell<Option<RegistrationId>>,
        /// Use a different file to store devices at.
        devices_file: RefCell<Option<PathBuf>>,
    }

    /// Save `model` to `storage` whenever `device` changed.
    fn save_device_automatically(storage: StorageServiceClient, model: ListStore, device: Device) {
        device.connect_notify_local(None, move |device, _| {
            glib::debug!("Device {} was changed, saving devices", device.label());
            storage.request_save_device_store(&model);
        });
    }

    impl TurnOnApplication {
        /// Start saving changes to the model automatically.
        ///
        /// Monitor the device model for changes, and automatically persist
        /// devices to `storage` whenever the model changed.
        fn save_automatically(&self, storage: StorageServiceClient) {
            // Monitor existing devices for changes
            for device in &self.devices.registered_devices() {
                save_device_automatically(
                    storage.clone(),
                    self.devices.registered_devices(),
                    device.unwrap().downcast().unwrap(),
                );
            }
            // Monitor any newly added device for changes
            self.devices.registered_devices().connect_items_changed(
                move |model, pos, _, n_added| {
                    glib::debug!("Device list changed, saving devices");
                    storage.request_save_device_store(model);
                    for n in pos..n_added {
                        save_device_automatically(
                            storage.clone(),
                            model.clone(),
                            model.item(n).unwrap().downcast().unwrap(),
                        );
                    }
                },
            );
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TurnOnApplication {
        const NAME: &'static str = "TurnOnApplication";

        type Type = super::TurnOnApplication;

        type ParentType = adw::Application;

        fn new() -> Self {
            Self {
                devices: Devices::default(),
                registered_search_provider: Default::default(),
                devices_file: Default::default(),
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TurnOnApplication {
        fn constructed(&self) {
            self.parent_constructed();

            let app = self.obj();
            app.add_main_option(
                "add-device",
                0.into(),
                OptionFlags::NONE,
                OptionArg::None,
                &dpgettext2(None, "option.add-device.description", "Add a new device"),
                None,
            );
            app.add_main_option(
                "turn-on-device",
                0.into(),
                OptionFlags::NONE,
                OptionArg::String,
                &dpgettext2(
                    None,
                    "option.turn-on-device.description",
                    "Turn on a device by its label",
                ),
                Some(&dpgettext2(
                    None,
                    "option.turn-on-device.arg.description",
                    "LABEL",
                )),
            );
            app.add_main_option(
                "list-devices",
                0.into(),
                OptionFlags::NONE,
                OptionArg::None,
                &dpgettext2(
                    None,
                    "option.list-devices.description",
                    "List all devices and their status",
                ),
                None,
            );
            app.add_main_option(
                "devices-file",
                0.into(),
                OptionFlags::NONE,
                OptionArg::Filename,
                &dpgettext2(
                    None,
                    "option.devices-file.description",
                    "Use the given file as storage for devices (for development only)",
                ),
                None,
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
            glib::debug!("Application starting");
            gtk::Window::set_default_icon_name(super::APP_ID);

            app.setup_actions();

            let devices_file = self.devices_file.borrow_mut().take().unwrap_or_else(|| {
                glib::user_data_dir()
                    .join(super::APP_ID)
                    .join("devices.json")
            });
            glib::debug!("Initializing storage from {}", devices_file.display());
            let storage = StorageService::new(devices_file);
            glib::info!("Loading devices synchronously");
            let devices = match storage.load_sync() {
                Err(error) => {
                    glib::warn!(
                        "Failed to load devices from {}: {}",
                        storage.target().display(),
                        error
                    );
                    Vec::new()
                }
                Ok(devices) => devices.into_iter().map(Device::from).collect(),
            };
            self.devices.registered_devices().remove_all();
            self.devices
                .registered_devices()
                .extend_from_slice(devices.as_slice());
            self.save_automatically(storage.client());
            glib::spawn_future_local(storage.spawn());

            glib::info!("Registering search provider");
            self.registered_search_provider
                .replace(register_app_search_provider(app.clone()));
        }

        /// Activate the application.
        ///
        /// Presents the current active window of the application, or creates a
        /// new application window and presents it, if the application doesn't
        /// have an active window currently.
        fn activate(&self) {
            glib::debug!("Activating application");
            self.parent_activate();
            let app: &super::TurnOnApplication = &self.obj();
            match app.active_window() {
                Some(window) => {
                    glib::debug!("Representing existing application window");
                    window.present()
                }
                None => {
                    glib::debug!("Creating new application window");
                    let window = TurnOnApplicationWindow::new(app);
                    window.bind_model(&self.devices);
                    window.present();
                }
            }
        }

        fn handle_local_options(&self, options: &glib::VariantDict) -> glib::ExitCode {
            glib::debug!("Handling local options");
            self.parent_handle_local_options(options);
            if let Ok(Some(path)) = options.lookup::<PathBuf>("devices-file") {
                glib::warn!(
                    "Overriding storage file to {}; only use for development purposes!",
                    path.display()
                );
                self.devices_file.replace(Some(path));
            }
            // -1 means continue normal command line processing
            glib::ExitCode::from(-1)
        }

        fn command_line(&self, command_line: &gtk::gio::ApplicationCommandLine) -> glib::ExitCode {
            let _guard = self.obj().hold();
            glib::debug!(
                "Handling command line. Remote? {}",
                command_line.is_remote()
            );
            let options = command_line.options_dict();
            if let Ok(Some(true)) = options.lookup("list-devices") {
                commandline::list_devices(&self.obj(), command_line)
            } else if let Ok(Some(true)) = options.lookup("add-device") {
                glib::debug!(
                    "Activating app.add-device action in response to command line argument"
                );
                // Activate application to show main window first
                self.obj().activate();
                self.obj().activate_action("add-device", None);
                glib::ExitCode::SUCCESS
            } else if let Ok(Some(label)) = options.lookup::<String>("turn-on-device") {
                commandline::turn_on_device_by_label(&self.obj(), command_line, label)
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
