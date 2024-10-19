// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::object::IsA;
use gtk::gio;
use gtk::glib;

use crate::model::Devices;

glib::wrapper! {
    pub struct TurnOnApplicationWindow(ObjectSubclass<imp::TurnOnApplicationWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap,
            gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
            gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl TurnOnApplicationWindow {
    /// Create a new application window for the given `application`.
    pub fn new(application: &impl IsA<gtk::Application>, devices: &Devices) -> Self {
        glib::Object::builder()
            .property("application", application)
            .property("devices", devices)
            .build()
    }
}

mod imp {
    use std::cell::RefCell;
    use std::error::Error;
    use std::time::Duration;

    use adw::subclass::prelude::*;
    use adw::{prelude::*, ToastOverlay};
    use futures_util::{select_biased, FutureExt, StreamExt, TryStreamExt};
    use gtk::glib::subclass::InitializingObject;
    use gtk::glib::Properties;
    use gtk::{glib, CompositeTemplate};

    use crate::i18n::gettext;
    use crate::model::{Device, Devices};
    use crate::net::{self, wol};
    use crate::widgets::device_row::DeviceRow;
    use crate::widgets::EditDeviceDialog;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::TurnOnApplicationWindow)]
    #[template(resource = "/de/swsnr/turnon/ui/turnon-application-window.ui")]
    pub struct TurnOnApplicationWindow {
        #[property(get, set, construct_only)]
        devices: RefCell<Devices>,
        #[template_child]
        devices_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        feedback: TemplateChild<ToastOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TurnOnApplicationWindow {
        const NAME: &'static str = "TurnOnApplicationWindow";

        type Type = super::TurnOnApplicationWindow;

        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("win.add_device", None, |window, _, _| {
                let dialog = EditDeviceDialog::new();
                dialog.connect_saved(glib::clone!(
                    #[weak]
                    window,
                    move |_, device| {
                        log::debug!("Adding new device: {:?}", device.imp());
                        window.devices().add_device(device);
                    }
                ));
                dialog.present(Some(window));
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl TurnOnApplicationWindow {
        fn turn_on_device(&self, device: Device) {
            let window = self.obj().clone();
            let mac_address = device.mac_addr6();
            log::info!(
                "Sending magic packet for mac address {mac_address} of device {}",
                device.label()
            );
            // Notify the user that we're about to send the magic packet to the target device
            let toast_sending = adw::Toast::builder()
                .title(gettext("Sending magic packet to device %s").replace("%s", &device.label()))
                .timeout(3)
                .build();
            window.imp().feedback.add_toast(toast_sending.clone());

            glib::spawn_future_local(glib::clone!(
                #[weak]
                window,
                #[weak_allow_none]
                toast_sending,
                async move {
                    let wol_timeout = Duration::from_secs(5);
                    let result: Result<(), Box<dyn Error>> = select_biased! {
                        result = wol(mac_address).fuse() => result,
                        _ = glib::timeout_future(wol_timeout).fuse() => {
                            Err(
                                std::io::Error::new(
                                    std::io::ErrorKind::TimedOut,
                                    format!("Failed to send magic packet within {wol_timeout:#?}")
                                ).into()
                            )
                        }
                    };
                    match result {
                        Ok(_) => {
                            toast_sending.inspect(|t| t.dismiss());
                            log::info!(
                                "Sent magic packet to {mac_address} of device {}",
                                device.label()
                            );
                            let toast = adw::Toast::builder()
                                .title(
                                    gettext("Sent magic packet to device %s")
                                        .replace("%s", &device.label()),
                                )
                                .timeout(3)
                                .build();
                            window.imp().feedback.add_toast(toast);
                        }
                        Err(error) => {
                            toast_sending.inspect(|t| t.dismiss());
                            log::warn!("Failed to send magic packet to {mac_address}: {error}");
                            let toast = adw::Toast::builder()
                                .title(
                                    gettext("Failed to send magic packet to device %s")
                                        .replace("%s", &device.label()),
                                )
                                .timeout(10)
                                .build();
                            window.imp().feedback.add_toast(toast);
                        }
                    }
                }
            ));
        }

        fn create_device_row(&self, object: &glib::Object) -> gtk::Widget {
            let device = &object.clone().downcast::<Device>().unwrap();
            let row = DeviceRow::new(device);
            // TODO: Restart monitoring if the target of a device changed!
            glib::spawn_future_local(
                net::monitor(device.host().into(), Duration::from_secs(5))
                    .map(Ok)
                    .try_for_each(glib::clone!(
                        #[weak]
                        row,
                        #[upgrade_or]
                        futures_util::future::err(()),
                        move |is_online| {
                            row.set_is_device_online(is_online);
                            futures_util::future::ok(())
                        }
                    )),
            );
            row.connect_activated(glib::clone!(
                #[strong(rename_to=window)]
                self.obj(),
                move |row| window.imp().turn_on_device(row.device())
            ));
            row.connect_deleted(glib::clone!(
                #[strong(rename_to=window)]
                self.obj(),
                move |_, device| {
                    log::info!("Deleting device {}", device.label());
                    window.devices().delete_device(device);
                }
            ));
            row.upcast()
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TurnOnApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let window = self.obj().clone();

            self.devices_list
                .get()
                .bind_model(Some(&self.devices.borrow().clone()), move |o| {
                    window.imp().create_device_row(o)
                });
        }
    }

    impl WidgetImpl for TurnOnApplicationWindow {}

    impl WindowImpl for TurnOnApplicationWindow {}

    impl ApplicationWindowImpl for TurnOnApplicationWindow {}

    impl AdwApplicationWindowImpl for TurnOnApplicationWindow {}
}
