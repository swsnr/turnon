// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::gio;
use gtk::glib;

use crate::model::Devices;

glib::wrapper! {
    pub struct WakeUpApplicationWindow(ObjectSubclass<imp::WakeUpApplicationWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap,
            gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
            gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl WakeUpApplicationWindow {
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

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::glib::subclass::InitializingObject;
    use gtk::glib::Properties;
    use gtk::{glib, CompositeTemplate};

    use crate::model::{Device, Devices};
    use crate::widgets::AddDeviceDialog;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::WakeUpApplicationWindow)]
    #[template(resource = "/de/swsnr/wakeup/ui/wakeup-application-window.ui")]
    pub struct WakeUpApplicationWindow {
        #[property(get, set, construct_only)]
        devices: RefCell<Devices>,
        #[template_child]
        devices_list: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WakeUpApplicationWindow {
        const NAME: &'static str = "WakeUpApplicationWindow";

        type Type = super::WakeUpApplicationWindow;

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

    #[glib::derived_properties]
    impl ObjectImpl for WakeUpApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();

            self.devices_list
                .get()
                .bind_model(Some(&self.devices.borrow().clone()), |item| {
                    let device = item.clone().downcast::<Device>().unwrap();
                    log::debug!("Creating label for device {:?}", device.imp());
                    gtk::Label::new(Some(&device.label())).upcast()
                });
        }
    }

    impl WidgetImpl for WakeUpApplicationWindow {}

    impl WindowImpl for WakeUpApplicationWindow {}

    impl ApplicationWindowImpl for WakeUpApplicationWindow {}

    impl AdwApplicationWindowImpl for WakeUpApplicationWindow {}
}
