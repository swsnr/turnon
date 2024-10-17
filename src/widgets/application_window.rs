// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

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
    pub fn new(application: &adw::Application, devices: &Devices) -> Self {
        glib::Object::builder()
            .property("application", application)
            .property("devices", devices)
            .build()
    }
}

mod imp {
    use std::cell::RefCell;
    use std::time::Duration;

    use adw::subclass::prelude::*;
    use adw::{prelude::*, ToastOverlay};
    use futures_util::{StreamExt, TryStreamExt};
    use gtk::glib::subclass::InitializingObject;
    use gtk::glib::Properties;
    use gtk::{glib, CompositeTemplate};

    use crate::i18n::gettext;
    use crate::model::{Device, Devices};
    use crate::net::{self, wol};
    use crate::widgets::device_row::DeviceRow;
    use crate::widgets::AddDeviceDialog;

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
                let dialog = AddDeviceDialog::new();
                dialog.connect_added(glib::clone!(
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
        fn create_device_row(&self, object: &glib::Object) -> gtk::Widget {
            let window = self.obj().clone();
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
            row.connect_activated(move |row| {
                let device = row.device();
                let mac_address = device.mac_addr6();
                log::info!(
                    "Sending magic packet for mac address {mac_address} of device {}",
                    device.label()
                );
                let toast_sending = adw::Toast::builder()
                    .title(
                        gettext("Sening magic packet to device %s").replace("%s", &device.label()),
                    )
                    .timeout(3)
                    .build();
                window.imp().feedback.add_toast(toast_sending.clone());
                glib::spawn_future_local(glib::clone!(
                    #[weak]
                    window,
                    #[weak_allow_none]
                    toast_sending,
                    async move {
                        match wol(mac_address).await {
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
            });
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
