// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::clone;
use gtk::{glib, prelude::ObjectExt};

use crate::model::Device;

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

    pub fn connect_added<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &Device) + 'static,
    {
        self.connect_local(
            "added",
            false,
            clone!(
                #[weak(rename_to=dialog)]
                &self,
                #[upgrade_or_default]
                move |args| {
                    let device = &args[1].get().expect("No device passed as signal argument?");
                    callback(&dialog, device);
                    None
                }
            ),
        )
    }
}

impl Default for AddDeviceDialog {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use std::cell::{Cell, RefCell};
    use std::net::IpAddr;
    use std::str::FromStr;
    use std::sync::LazyLock;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::glib;
    use gtk::glib::subclass::{InitializingObject, Signal};
    use gtk::glib::Properties;
    use gtk::CompositeTemplate;
    use macaddr::MacAddr6;

    use crate::model::Device;
    use crate::widgets::ValidationIndicator;

    #[derive(CompositeTemplate, Properties)]
    #[template(resource = "/de/swsnr/turnon/ui/add-device-dialog.ui")]
    #[properties(wrapper_type = super::AddDeviceDialog)]
    pub struct AddDeviceDialog {
        #[property(get, set)]
        pub label: RefCell<String>,
        #[property(get)]
        pub label_valid: Cell<bool>,
        #[property(get, set)]
        pub mac_address: RefCell<String>,
        #[property(get)]
        pub mac_address_valid: Cell<bool>,
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
            self.label_valid.set(self.is_label_valid());
            self.obj().notify_label_valid();
            self.obj().notify_is_valid();
        }

        fn is_mac_address_valid(&self) -> bool {
            let text = self.mac_address.borrow();
            !text.is_empty() && macaddr::MacAddr::from_str(&text).is_ok()
        }

        fn validate_mac_address(&self) {
            self.mac_address_valid.set(self.is_mac_address_valid());
            self.obj().notify_mac_address_valid();
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
            self.obj().notify_is_valid();
        }

        fn host_valid(&self) -> bool {
            *self.host_indicator.borrow() != "empty"
        }

        fn validate_all(&self) {
            self.validate_label();
            self.validate_mac_address();
            self.validate_host();
        }

        fn is_valid(&self) -> bool {
            self.label_valid.get() && self.mac_address_valid.get() && self.host_valid()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AddDeviceDialog {
        const NAME: &'static str = "AddDeviceDialog";

        type Type = super::AddDeviceDialog;

        type ParentType = adw::Dialog;

        fn new() -> Self {
            Self {
                label: Default::default(),
                label_valid: Default::default(),
                mac_address: Default::default(),
                mac_address_valid: Default::default(),
                host: Default::default(),
                host_indicator: RefCell::new("empty".to_string()),
                is_valid: (),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            ValidationIndicator::ensure_type();
            Device::ensure_type();

            klass.bind_template();

            klass.install_action("device.add", None, |dialog, _, _| {
                if dialog.is_valid() {
                    // At this point we know that the MAC address is valid, hence we can unwrap
                    let mac_address = MacAddr6::from_str(&dialog.mac_address()).unwrap();
                    let device =
                        Device::new(dialog.label().clone(), mac_address, dialog.host().clone());
                    dialog.emit_by_name::<()>("added", &[&device]);
                    dialog.close();
                }
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AddDeviceDialog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| {
                vec![Signal::builder("added")
                    .action()
                    .param_types([Device::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.validate_all();
            self.obj().action_set_enabled("device.add", false);
            self.obj().connect_label_notify(|dialog| {
                dialog.imp().validate_label();
            });
            self.obj().connect_mac_address_notify(|dialog| {
                dialog.imp().validate_mac_address();
            });
            self.obj().connect_host_notify(|dialog| {
                dialog.imp().validate_host();
            });
            self.obj().connect_is_valid_notify(|dialog| {
                dialog.action_set_enabled("device.add", dialog.is_valid());
            });
        }
    }

    impl WidgetImpl for AddDeviceDialog {}

    impl AdwDialogImpl for AddDeviceDialog {}
}
