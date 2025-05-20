// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use glib::object::ObjectExt;
use gtk::glib;

pub use self::r#enum::MoveDirection;
use super::super::model::Device;

#[allow(clippy::as_conversions, reason = "Comes from glib::Enum")]
mod r#enum {
    /// The direction a device was moved into.
    #[derive(Debug, Clone, Copy, Eq, PartialEq, glib::Enum)]
    #[enum_type(name = "DeviceMoveDirection")]
    pub enum MoveDirection {
        /// The device was moved upwards.
        Upwards,
        /// The device was moved downwards.
        Downwards,
    }
}

glib::wrapper! {
    pub struct DeviceRow(ObjectSubclass<imp::DeviceRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBox, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl DeviceRow {
    pub fn new(device: &Device) -> Self {
        glib::Object::builder()
            .property("device", device)
            .property("is-device-online", false)
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
                    let device = args
                        .get(1)
                        .expect("'deleted' signal expected one argument but got none!")
                        .get()
                        .unwrap_or_else(|error| {
                            panic!("'deleted' signal expected Device as first argument: {error}");
                        });
                    callback(&row, device);
                    None
                }
            ),
        )
    }

    pub fn connect_added<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &Device) + 'static,
    {
        self.connect_local(
            "added",
            false,
            glib::clone!(
                #[weak(rename_to=row)]
                &self,
                #[upgrade_or_default]
                move |args| {
                    let device = args
                        .get(1)
                        .expect("'added' signal expects one argument but got none?")
                        .get()
                        .unwrap_or_else(|error| {
                            panic!("'added' signal expected Device as first argument: {error}");
                        });
                    callback(&row, device);
                    None
                }
            ),
        )
    }

    pub fn connect_moved<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &Device, MoveDirection) + 'static,
    {
        self.connect_local(
            "moved",
            false,
            glib::clone!(
                #[weak(rename_to=row)]
                &self,
                #[upgrade_or_default]
                move |args| {
                    let device = args
                        .get(1)
                        .expect("'moved' signal expected two arguments but got none")
                        .get()
                        .unwrap_or_else(|error| {
                            panic!("'moved' signal expected Device as first argument: {error}");
                        });
                    let direction = args
                        .get(2)
                        .expect("'moved' signal expected two arguments but got only one")
                        .get()
                        .unwrap_or_else(|error| {
                            panic!(
                                "'moved' signal expected MoveDirection as second argument: {error}"
                            );
                        });
                    callback(&row, device, direction);
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
    use glib::Properties;
    use glib::subclass::{InitializingObject, Signal};
    use gtk::gdk::{Key, ModifierType};
    use gtk::{CompositeTemplate, template_callbacks};

    use crate::app::model::Device;

    use super::super::EditDeviceDialog;
    use super::MoveDirection;

    #[derive(CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::DeviceRow)]
    #[template(resource = "/de/swsnr/turnon/ui/device-row.ui")]
    pub struct DeviceRow {
        #[property(get, set)]
        device: RefCell<Device>,
        #[property(get, set)]
        is_device_online: Cell<bool>,
        #[property(get, set, nullable)]
        device_url: RefCell<Option<String>>,
        #[property(get)]
        suffix_mode: RefCell<String>,
    }

    #[template_callbacks]
    impl DeviceRow {
        #[template_callback(function)]
        pub fn device_mac_address(device: &Device) -> String {
            device.mac_address().to_string()
        }

        #[template_callback(function)]
        pub fn device_state_name(is_device_online: bool) -> &'static str {
            if is_device_online {
                "online"
            } else {
                "offline"
            }
        }

        #[template_callback(function)]
        pub fn device_host(host: &str, url: Option<&str>) -> String {
            if let Some(url) = url {
                format!(
                    "<a href=\"{}\">{}</a>",
                    glib::markup_escape_text(url),
                    glib::markup_escape_text(host)
                )
            } else {
                glib::markup_escape_text(host).to_string()
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

            klass.install_action("row.move-up", None, |row, _, _| {
                row.emit_by_name::<()>("moved", &[&row.device(), &MoveDirection::Upwards]);
            });
            klass.install_action("row.move-down", None, |row, _, _| {
                row.emit_by_name::<()>("moved", &[&row.device(), &MoveDirection::Downwards]);
            });
            klass.install_action("row.ask-delete", None, |row, _, _| {
                row.imp().set_suffix_mode("confirm-delete");
            });
            klass.install_action("row.cancel-delete", None, |row, _, _| {
                row.imp().set_suffix_mode("buttons");
            });
            klass.install_action("row.delete", None, |row, _, _| {
                row.emit_by_name::<()>("deleted", &[&row.device()]);
            });
            klass.install_action("row.edit", None, |obj, _, _| {
                let dialog = EditDeviceDialog::edit(obj.device());
                dialog.present(Some(obj));
            });
            klass.install_action("row.add", None, |row, _, _| {
                // Create a fresh device, edit it, and then emit an added signal
                // if the user saves the device.
                let current_device = row.device();
                let dialog = EditDeviceDialog::edit(Device::new(
                    &current_device.label(),
                    current_device.mac_address(),
                    &current_device.host(),
                ));
                dialog.connect_saved(glib::clone!(
                    #[weak]
                    row,
                    move |_, device| row.emit_by_name::<()>("added", &[device])
                ));
                dialog.present(Some(row));
            });

            klass.add_binding_action(Key::Up, ModifierType::ALT_MASK, "row.move-up");
            klass.add_binding_action(Key::Down, ModifierType::ALT_MASK, "row.move-down");
            klass.add_binding_action(Key::Return, ModifierType::ALT_MASK, "row.edit");
            klass.add_binding_action(Key::N, ModifierType::CONTROL_MASK, "row.add");
            klass.add_binding_action(
                Key::Delete,
                ModifierType::NO_MODIFIER_MASK,
                "row.ask-delete",
            );
            klass.add_binding_action(Key::Delete, ModifierType::CONTROL_MASK, "row.delete");
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            Self {
                device: RefCell::default(),
                is_device_online: Cell::default(),
                device_url: RefCell::default(),
                suffix_mode: RefCell::new("buttons".into()),
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DeviceRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| {
                vec![
                    Signal::builder("deleted")
                        .action()
                        .param_types([Device::static_type()])
                        .build(),
                    Signal::builder("added")
                        .action()
                        .param_types([Device::static_type()])
                        .build(),
                    Signal::builder("moved")
                        .action()
                        .param_types([Device::static_type(), MoveDirection::static_type()])
                        .build(),
                ]
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
