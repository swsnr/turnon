// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::subclass::prelude::*;
use glib::object::IsA;
use gtk::gio;
use gtk::glib;

use crate::app::model::Devices;

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

    pub fn bind_model(&self, devices: &Devices) {
        self.imp().bind_model(devices);
    }
}

mod imp {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use adw::subclass::prelude::*;
    use adw::{Toast, ToastOverlay, prelude::*};
    use futures_util::{StreamExt, TryStreamExt, stream};
    use glib::dpgettext2;
    use gtk::CompositeTemplate;
    use gtk::gdk::{Key, ModifierType};
    use gtk::glib::subclass::InitializingObject;

    use crate::app::model::{Device, Devices};
    use crate::app::widgets::MoveDirection;
    use crate::config::G_LOG_DOMAIN;
    use crate::net::{self, probe_http};

    use super::super::DeviceRow;

    #[derive(Default, CompositeTemplate, glib::Properties)]
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
                    if let Ok(()) = device.wol().await {
                        toast_sending.inspect(Toast::dismiss);

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
                    } else {
                        toast_sending.inspect(Toast::dismiss);
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
            ));
        }

        fn monitor_device(row: &DeviceRow) -> stream::AbortHandle {
            let device = row.device();
            let (monitor, abort_monitoring) = stream::abortable(
                net::monitor(device.host().into(), Duration::from_secs(5)).map(Ok),
            );

            // Start monitoring the device, but only as long as the row still exists.
            // To this end only hold a weak reference to the row in the stream,
            // and for every ping explicitly attempt to upgrade to a strong reference.
            // If the upgrade fails return an error to exit the whole stream.
            let row = row.downgrade();
            glib::spawn_future_local(monitor.try_for_each(move |result| {
                let row = row.clone();
                async move {
                    if let Some(row) = row.upgrade() {
                        match result {
                            Ok(_) => {
                                row.set_is_device_online(true);
                                let url = probe_http(&row.device().host(), Duration::from_secs(1))
                                    .await
                                    .map(|scheme| scheme.to_url(&row.device().host()))
                                    .inspect(|url| {
                                        glib::info!(
                                            "Device {} offers URL {url}",
                                            row.device().label()
                                        );
                                    });
                                row.set_device_url(url);
                            }
                            Err(error) => {
                                glib::trace!(
                                    "Device {} not reachable: {error}",
                                    row.device().label()
                                );
                                row.set_device_url(None::<String>);
                                row.set_is_device_online(false);
                            }
                        }
                        Ok(())
                    } else {
                        Err(())
                    }
                }
            }));
            abort_monitoring
        }

        fn create_device_row(&self, devices: &Devices, object: &glib::Object) -> gtk::Widget {
            let device = &object.clone().downcast::<Device>().unwrap();
            let row = DeviceRow::new(device);
            let ongoing_monitor = Rc::new(RefCell::new(Self::monitor_device(&row)));

            // If the host changed monitor the new host.
            device.connect_host_notify(glib::clone!(
                #[strong]
                ongoing_monitor,
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
                ongoing_monitor,
                #[strong]
                devices,
                move |_, device| {
                    glib::info!("Deleting device {}", device.label());
                    // Abort monitoring if the device is deleted
                    ongoing_monitor.borrow().abort();
                    if let Some(index) = devices.registered_devices().find(device) {
                        devices.registered_devices().remove(index);
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
            row.connect_moved(glib::clone!(
                #[strong]
                devices,
                move |_, device, direction| {
                    let devices = devices.registered_devices();
                    let offset = match direction {
                        MoveDirection::Upwards => -1,
                        MoveDirection::Downwards => 1,
                    };
                    if let Some(current_index) = devices.find(device) {
                        let swap_index = i64::from(current_index) + offset;
                        if 0 <= swap_index && swap_index < i64::from(devices.n_items()) {
                            if let Some(device_swapped) =
                                devices.item(u32::try_from(swap_index).unwrap())
                            {
                                // We remove the other device, not the device being moved; this
                                // retains the widget for the device being moved in views consuming
                                // the model, meaning it remains focused, and we can repeatedly
                                // move the same device to rearrange it.
                                devices.remove(u32::try_from(swap_index).unwrap());
                                devices.insert(current_index, &device_swapped);
                            }
                        }
                    }
                }
            ));

            let is_registered = devices.registered_devices().find(device).is_some();
            row.action_set_enabled("row.ask-delete", is_registered);
            row.action_set_enabled("row.delete", is_registered);
            row.action_set_enabled("row.edit", is_registered);
            row.action_set_enabled("row.add", !is_registered);
            row.action_set_enabled("row.move-up", is_registered);
            row.action_set_enabled("row.move-down", is_registered);
            if !is_registered {
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
            klass.add_binding_action(Key::N, ModifierType::CONTROL_MASK, "app.add-device");
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
