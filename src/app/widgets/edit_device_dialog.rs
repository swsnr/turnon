// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use glib::clone;
use gtk::{glib, prelude::ObjectExt};

use crate::app::model::Device;

glib::wrapper! {
    pub struct EditDeviceDialog(ObjectSubclass<imp::EditDeviceDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl EditDeviceDialog {
    /// Create a new dialog to edit a new device.
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    /// Create a new dialog the edit an existing device.
    pub fn edit(device: Device) -> Self {
        glib::Object::builder()
            .property("device", Some(device))
            .build()
    }

    pub fn connect_saved<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &Device) + 'static,
    {
        self.connect_local(
            "saved",
            false,
            clone!(
                #[weak(rename_to=dialog)]
                &self,
                #[upgrade_or_default]
                move |args| {
                    let device = args
                        .get(1)
                        .expect("'saved' signal expects one argument but got none?")
                        .get()
                        .unwrap_or_else(|error| {
                            panic!("'saved' signal expected Device as first argument: {error}");
                        });
                    callback(&dialog, device);
                    None
                }
            ),
        )
    }
}

impl Default for EditDeviceDialog {
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
    use gtk::CompositeTemplate;
    use gtk::gdk::{Key, ModifierType};
    use gtk::glib::Properties;
    use gtk::glib::subclass::{InitializingObject, Signal};
    use gtk::{glib, template_callbacks};

    use crate::app::model::Device;
    use crate::net::{MacAddr6Boxed, SocketAddrV4Boxed, WOL_DEFAULT_TARGET_ADDRESS};

    use super::super::ValidationIndicator;

    /// Whether `s` looks as if it's a host and port, e.g. `localhost:1245`.
    fn is_host_and_port(s: &str) -> bool {
        if let Some((_, port)) = s.rsplit_once(':') {
            port.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    #[derive(CompositeTemplate, Properties)]
    #[template(resource = "/de/swsnr/turnon/ui/edit-device-dialog.ui")]
    #[properties(wrapper_type = super::EditDeviceDialog)]
    pub struct EditDeviceDialog {
        #[property(get, set, construct_only)]
        pub device: RefCell<Option<Device>>,
        #[property(get, set)]
        pub label: RefCell<String>,
        #[property(get)]
        pub label_valid: Cell<bool>,
        #[property(get, set)]
        pub mac_address: RefCell<String>,
        #[property(get)]
        pub mac_address_valid: Cell<bool>,
        #[property(get, set)]
        pub target_address: RefCell<String>,
        #[property(get)]
        pub target_address_valid: Cell<bool>,
        #[property(get, set)]
        pub host: RefCell<String>,
        #[property(get, default = "invalid-empty")]
        pub host_indicator: RefCell<String>,
        #[property(get = Self::is_valid, default = false, type = bool)]
        pub is_valid: (),
    }

    #[template_callbacks]
    impl EditDeviceDialog {
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

        fn is_target_address_valid(&self) -> bool {
            let text = self.target_address.borrow();
            !text.is_empty() && SocketAddrV4Boxed::from_str(&text).is_ok()
        }

        fn validate_mac_address(&self) {
            self.mac_address_valid.set(self.is_mac_address_valid());
            self.obj().notify_mac_address_valid();
            self.obj().notify_is_valid();
        }

        fn validate_target_address(&self) {
            self.target_address_valid
                .set(self.is_target_address_valid());
            self.obj().notify_target_address_valid();
            self.obj().notify_is_valid();
        }

        fn validate_host(&self) {
            let host = self.host.borrow();
            let indicator = match IpAddr::from_str(&host) {
                Ok(IpAddr::V4(..)) => "ipv4",
                Ok(IpAddr::V6(..)) => "ipv6",
                Err(_) => {
                    if host.is_empty() {
                        "invalid-empty"
                    } else if is_host_and_port(&host) {
                        // Check whether the user specified a port, and if so,
                        // reject the input.
                        //
                        // See https://codeberg.org/swsnr/turnon/issues/40
                        "invalid-socket-address"
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
            !self.host_indicator.borrow().starts_with("invalid-")
        }

        fn validate_all(&self) {
            self.validate_label();
            self.validate_mac_address();
            self.validate_host();
            self.validate_target_address();
        }

        fn is_valid(&self) -> bool {
            self.label_valid.get() && self.mac_address_valid.get() && self.host_valid()
        }

        #[template_callback]
        fn move_to_next_entry(entry: &adw::EntryRow) {
            entry.emit_move_focus(gtk::DirectionType::TabForward);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EditDeviceDialog {
        const NAME: &'static str = "TurnOnEditDeviceDialog";

        type Type = super::EditDeviceDialog;

        type ParentType = adw::Dialog;

        fn new() -> Self {
            Self {
                device: RefCell::default(),
                label: RefCell::default(),
                label_valid: Cell::default(),
                mac_address: RefCell::default(),
                mac_address_valid: Cell::default(),
                target_address: RefCell::default(),
                target_address_valid: Cell::default(),
                host: RefCell::default(),
                host_indicator: RefCell::new("invalid-empty".to_string()),
                is_valid: (),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            ValidationIndicator::ensure_type();
            Device::ensure_type();

            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action("device.save", None, |dialog, _, _| {
                if dialog.is_valid() {
                    // At this point we know that the addresses are valid, hence we can unwrap
                    let mac_address = MacAddr6Boxed::from_str(&dialog.mac_address()).unwrap();
                    let target_address =
                        SocketAddrV4Boxed::from_str(&dialog.target_address()).unwrap();
                    let device = match dialog.device() {
                        Some(device) => {
                            // The dialog edits an existing device, so update its fields.
                            device.set_label(dialog.label());
                            device.set_mac_address(mac_address);
                            device.set_host(dialog.host());
                            device.set_target_address(target_address);
                            device
                        }
                        None => {
                            // Create a new device if the dialog does not own a device.
                            Device::new(
                                &dialog.label(),
                                mac_address,
                                &dialog.host(),
                                target_address,
                            )
                        }
                    };
                    dialog.emit_by_name::<()>("saved", &[&device]);
                    dialog.close();
                }
            });

            klass.add_binding_action(Key::S, ModifierType::CONTROL_MASK, "device.save");
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EditDeviceDialog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| {
                vec![
                    Signal::builder("saved")
                        .action()
                        .param_types([Device::static_type()])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();
            if let Some(device) = self.obj().device() {
                // Initialize properties from device
                self.obj().set_label(device.label());
                self.obj().set_mac_address(device.mac_address().to_string());
                self.obj().set_host(device.host());
                self.obj()
                    .set_target_address(device.target_address().to_string());
            } else {
                // If this dialog doesn't edit an existing device, pre-fill a
                // reasonable default target address.
                self.obj()
                    .set_target_address(WOL_DEFAULT_TARGET_ADDRESS.to_string());
            }
            // After initialization, update validation status.
            self.validate_all();
            self.obj().action_set_enabled("device.save", false);
            self.obj().connect_label_notify(|dialog| {
                dialog.imp().validate_label();
            });
            self.obj().connect_mac_address_notify(|dialog| {
                dialog.imp().validate_mac_address();
            });
            self.obj().connect_host_notify(|dialog| {
                dialog.imp().validate_host();
            });
            self.obj().connect_target_address_notify(|dialog| {
                dialog.imp().validate_target_address();
            });
            self.obj().connect_is_valid_notify(|dialog| {
                dialog.action_set_enabled("device.save", dialog.is_valid());
            });
        }
    }

    impl WidgetImpl for EditDeviceDialog {}

    impl AdwDialogImpl for EditDeviceDialog {}
}
