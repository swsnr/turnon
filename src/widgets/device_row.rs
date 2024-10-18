// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::object::ObjectExt;
use gtk::glib;

use crate::model::Device;

glib::wrapper! {
    pub struct DeviceRow(ObjectSubclass<imp::DeviceRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBox, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl DeviceRow {
    pub fn new(device: &Device) -> Self {
        glib::Object::builder()
            .property("device", device)
            .property("is_device_online", false)
            .build()
    }

    pub fn connect_deleted<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &Device) + 'static,
    {
        self.connect_local(
            "deleted",
            false,
            glib::clone!(
                #[weak(rename_to=row)]
                &self,
                #[upgrade_or_default]
                move |args| {
                    let device = &args[1].get().expect("No device passed as signal argument?");
                    callback(&row, device);
                    None
                }
            ),
        )
    }
}

impl Default for DeviceRow {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

mod imp {
    use std::cell::{Cell, RefCell};
    use std::sync::LazyLock;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::subclass::{InitializingObject, Signal};
    use glib::Properties;
    use gtk::{template_callbacks, CompositeTemplate};

    use crate::model::Device;

    #[derive(CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::DeviceRow)]
    #[template(resource = "/de/swsnr/turnon/ui/device-row.ui")]
    pub struct DeviceRow {
        #[property(get, set)]
        device: RefCell<Device>,
        #[property(get, set)]
        is_device_online: Cell<bool>,
        #[property(get)]
        suffix_mode: RefCell<String>,
    }

    #[template_callbacks]
    impl DeviceRow {
        #[template_callback]
        pub fn device_mac_address(_row: &super::DeviceRow, device: &Device) -> String {
            device.mac_addr6().to_string()
        }

        #[template_callback]
        pub fn device_state_name(_row: &super::DeviceRow, is_device_online: bool) -> &'static str {
            if is_device_online {
                "online"
            } else {
                "offline"
            }
        }

        pub fn set_suffix_mode(&self, mode: &str) {
            self.suffix_mode.replace(mode.to_owned());
            self.obj().notify_suffix_mode();
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceRow {
        const NAME: &'static str = "DeviceRow";

        type Type = super::DeviceRow;

        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action("row.ask_delete", None, |obj, _, _| {
                obj.imp().set_suffix_mode("confirm-delete");
            });
            klass.install_action("row.cancel-delete", None, |obj, _, _| {
                obj.imp().set_suffix_mode("buttons");
            });
            klass.install_action("row.delete", None, |obj, _, _| {
                obj.emit_by_name::<()>("deleted", &[&obj.device()])
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            Self {
                device: Default::default(),
                is_device_online: Default::default(),
                suffix_mode: RefCell::new("buttons".into()),
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DeviceRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| {
                vec![Signal::builder("deleted")
                    .action()
                    .param_types([Device::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for DeviceRow {}

    impl ListBoxRowImpl for DeviceRow {}

    impl PreferencesRowImpl for DeviceRow {}

    impl ActionRowImpl for DeviceRow {}
}
