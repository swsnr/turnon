// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::glib;

glib::wrapper! {
    pub struct AddDeviceDialog(ObjectSubclass<imp::AddDeviceDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl AddDeviceDialog {
    /// Create a new dialog to add a device.
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

impl Default for AddDeviceDialog {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use std::cell::RefCell;
    use std::net::IpAddr;
    use std::str::FromStr;

    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::glib::prelude::*;
    use gtk::glib::subclass::InitializingObject;
    use gtk::glib::Properties;
    use gtk::CompositeTemplate;

    #[derive(CompositeTemplate, Properties)]
    #[template(resource = "/de/swsnr/wakeup/ui/add-device-dialog.ui")]
    #[properties(wrapper_type = super::AddDeviceDialog)]
    pub struct AddDeviceDialog {
        #[property(get, set)]
        pub label: RefCell<String>,
        #[property(get, default = "invalid")]
        pub label_indicator: RefCell<String>,
        #[property(get, set)]
        pub host: RefCell<String>,
        #[property(get, default = "empty")]
        pub host_indicator: RefCell<String>,
        #[property(get = Self::is_valid, default = false, type = bool)]
        pub is_valid: (),
    }

    impl AddDeviceDialog {
        fn is_label_valid(&self) -> bool {
            !self.label.borrow().is_empty()
        }

        fn validate_label(&self) {
            // These refer to the names of the stack pages in the label entry
            let indicator = if self.is_label_valid() {
                "valid"
            } else {
                "invalid"
            };
            self.label_indicator.replace(indicator.to_owned());
            self.obj().notify_label_indicator();
            self.obj().notify_is_valid();
        }

        fn validate_host(&self) {
            let host = self.host.borrow();
            let indicator = match IpAddr::from_str(&host) {
                Ok(IpAddr::V4(..)) => "ipv4",
                Ok(IpAddr::V6(..)) => "ipv6",
                Err(_) => {
                    if host.is_empty() {
                        "empty"
                    } else {
                        "host"
                    }
                }
            };
            self.host_indicator.replace(indicator.to_owned());
            self.obj().notify_host_indicator();
        }

        fn validate_all(&self) {
            self.validate_label();
            self.validate_host();
        }

        fn is_valid(&self) -> bool {
            return *self.label_indicator.borrow() == "valid";
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AddDeviceDialog {
        const NAME: &'static str = "AddDeviceDialog";

        type Type = super::AddDeviceDialog;

        type ParentType = adw::Dialog;

        fn new() -> Self {
            Self {
                label: RefCell::new(String::new()),
                label_indicator: RefCell::new("invalid".to_string()),
                host: RefCell::new(String::new()),
                host_indicator: RefCell::new("empty".to_string()),
                is_valid: (),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AddDeviceDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.validate_all();
            self.obj().connect_label_notify(|dialog| {
                dialog.imp().validate_label();
            });
            self.obj().connect_host_notify(|dialog| {
                dialog.imp().validate_host();
            });
        }
    }

    impl WidgetImpl for AddDeviceDialog {}

    impl AdwDialogImpl for AddDeviceDialog {}
}
