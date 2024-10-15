// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

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
}

impl Default for DeviceRow {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use glib::Properties;
    use gtk::{template_callbacks, CompositeTemplate};

    use crate::model::Device;

    #[derive(CompositeTemplate, Default, Properties)]
    #[properties(wrapper_type = super::DeviceRow)]
    #[template(resource = "/de/swsnr/turnon/ui/device-row.ui")]
    pub struct DeviceRow {
        #[property(get, set)]
        device: RefCell<Device>,
        #[property(get, set)]
        is_device_online: Cell<bool>,
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceRow {
        const NAME: &'static str = "DeviceRow";

        type Type = super::DeviceRow;

        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DeviceRow {}

    impl WidgetImpl for DeviceRow {}

    impl ListBoxRowImpl for DeviceRow {}

    impl PreferencesRowImpl for DeviceRow {}

    impl ActionRowImpl for DeviceRow {}
}
