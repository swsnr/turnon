// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::object::IsA;
use gtk::gio::ActionEntry;
use gtk::gio::{self, PropertyAction};
use gtk::glib;

use crate::app::model::Devices;
use crate::app::TurnOnApplication;
use crate::config::G_LOG_DOMAIN;

use super::EditDeviceDialog;

glib::wrapper! {
    pub struct TurnOnApplicationWindow(ObjectSubclass<imp::TurnOnApplicationWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap,
            gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
            gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl TurnOnApplicationWindow {
    /// Create a new application window for the given `application`.
    pub fn new(application: &impl IsA<gtk::Application>, startpage_icon_name: &str) -> Self {
        glib::Object::builder()
            .property("application", application)
            .property("startpage-icon-name", startpage_icon_name)
            .build()
    }

    pub fn application(&self) -> TurnOnApplication {
        GtkWindowExt::application(self)
            .unwrap()
            .downcast::<TurnOnApplication>()
            .unwrap()
    }

    pub fn bind_model(&self, devices: &Devices) {
        self.setup_actions();
        self.imp().bind_model(devices);
    }

    fn setup_actions(&self) {
        let add_device = ActionEntry::builder("add-device")
            .activate(move |window: &TurnOnApplicationWindow, _, _| {
                let dialog = EditDeviceDialog::new();
                dialog.connect_saved(glib::clone!(
                    #[weak(rename_to = devices)]
                    window.application().devices(),
                    move |_, device| {
                        glib::debug!("Adding new device: {:?}", device.imp());
                        devices.registered_devices().append(device);
                    }
                ));
                dialog.present(Some(window));
            })
            .build();

        self.add_action_entries([add_device]);

        let scan_network = PropertyAction::new(
            "toggle-scan-network",
            &self.application().devices().discovered_devices(),
            "discovery-enabled",
        );
        self.add_action(&scan_network);
    }
}

mod imp {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use adw::subclass::prelude::*;
    use adw::{prelude::*, ToastOverlay};
    use futures_util::{stream, StreamExt, TryStreamExt};
    use glib::dpgettext2;
    use gtk::glib::subclass::InitializingObject;
    use gtk::{glib, CompositeTemplate};

    use crate::app::model::{Device, Devices};
    use crate::config::G_LOG_DOMAIN;
    use crate::net;

    use super::super::DeviceRow;

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[properties(wrapper_type = super::TurnOnApplicationWindow)]
    #[template(resource = "/de/swsnr/turnon/ui/turnon-application-window.ui")]
    pub struct TurnOnApplicationWindow {
        #[property(get, set)]
        startpage_icon_name: RefCell<String>,
        #[template_child]
        devices_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        feedback: TemplateChild<ToastOverlay>,
    }

    impl TurnOnApplicationWindow {
        pub fn bind_model(&self, model: &Devices) {
            self.devices_list.get().bind_model(
                Some(model),
                glib::clone!(
                    #[strong(rename_to = window)]
                    self.obj(),
                    #[strong]
                    model,
                    move |o| window.imp().create_device_row(&model, o)
                ),
            );
        }

        fn turn_on_device(&self, device: Device) {
            let window = self.obj().clone();
            // Notify the user that we're about to send the magic packet to the target device
            let toast_sending = adw::Toast::builder()
                .title(
                    dpgettext2(
                        None,
                        "application-window.feedback.toast",
                        "Sending magic packet to device %s",
                    )
                    .replace("%s", &device.label()),
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
                    match device.wol().await {
                        Ok(_) => {
                            toast_sending.inspect(|t| t.dismiss());

                            let toast = adw::Toast::builder()
                                .title(
                                    dpgettext2(
                                        None,
                                        "application-window.feedback.toast",
                                        "Sent magic packet to device %s",
                                    )
                                    .replace("%s", &device.label()),
                                )
                                .timeout(3)
                                .build();
                            window.imp().feedback.add_toast(toast);
                        }
                        Err(_) => {
                            toast_sending.inspect(|t| t.dismiss());
                            let toast = adw::Toast::builder()
                                .title(
                                    dpgettext2(
                                        None,
                                        "application-window.feedback.toast",
                                        "Failed to send magic packet to device %s",
                                    )
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

        fn monitor_device(row: &DeviceRow) -> stream::AbortHandle {
            let device = row.device();
            let (monitor, abort_monitoring) = stream::abortable(
                net::monitor(device.host().into(), Duration::from_secs(5)).map(Ok),
            );
            glib::spawn_future_local(monitor.try_for_each(glib::clone!(
                #[weak]
                row,
                #[upgrade_or]
                futures_util::future::err(()),
                move |result| {
                    if let Err(error) = &result {
                        glib::trace!("Device {} not reachable: {error}", row.device().label());
                    }
                    row.set_is_device_online(result.is_ok());
                    futures_util::future::ok(())
                }
            )));
            abort_monitoring
        }

        fn create_device_row(&self, devices: &Devices, object: &glib::Object) -> gtk::Widget {
            let device = &object.clone().downcast::<Device>().unwrap();
            let row = DeviceRow::new(device);
            let ongoing_monitor = Rc::new(RefCell::new(Self::monitor_device(&row)));
            // If the host changed monitor the new host.
            device.connect_host_notify(glib::clone!(
                #[weak]
                row,
                move |_| {
                    let previous_monitor = ongoing_monitor.replace(Self::monitor_device(&row));
                    previous_monitor.abort();
                },
            ));
            row.connect_activated(glib::clone!(
                #[strong(rename_to=window)]
                self.obj(),
                move |row| window.imp().turn_on_device(row.device())
            ));
            row.connect_deleted(glib::clone!(
                #[strong]
                devices,
                move |_, device| {
                    glib::info!("Deleting device {}", device.label());
                    if let Some(index) = devices.registered_devices().find(device) {
                        devices.registered_devices().remove(index)
                    }
                }
            ));
            row.connect_added(glib::clone!(
                #[strong]
                devices,
                move |_, device| {
                    glib::info!("Adding device {}", device.label());
                    devices.registered_devices().append(device);
                }
            ));

            if devices.registered_devices().find(device).is_some() {
                row.set_can_delete(true);
                row.set_can_edit(true);
            } else {
                row.set_can_add(true);
                row.add_css_class("discovered");
            }

            row.upcast()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TurnOnApplicationWindow {
        const NAME: &'static str = "TurnOnApplicationWindow";

        type Type = super::TurnOnApplicationWindow;

        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TurnOnApplicationWindow {}

    impl WidgetImpl for TurnOnApplicationWindow {}

    impl WindowImpl for TurnOnApplicationWindow {}

    impl ApplicationWindowImpl for TurnOnApplicationWindow {}

    impl AdwApplicationWindowImpl for TurnOnApplicationWindow {}
}
