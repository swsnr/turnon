// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::gio;

glib::wrapper! {
    pub struct Devices(ObjectSubclass<imp::Devices>) @implements gio::ListModel;
}

impl Default for Devices {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

mod imp {
    use glib::types::StaticType;
    use gtk::gio;
    use gtk::gio::prelude::*;
    use gtk::gio::subclass::prelude::*;

    use super::super::Device;

    #[derive(Debug, glib::Properties)]
    #[properties(wrapper_type = super::Devices)]
    pub struct Devices {
        #[property(get)]
        pub registered_devices: gio::ListStore,
        #[property(get)]
        pub discovered_devices: gio::ListStore,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Devices {
        const NAME: &'static str = "Devices";

        type Type = super::Devices;

        type Interfaces = (gio::ListModel,);

        fn new() -> Self {
            Self {
                registered_devices: gio::ListStore::with_type(Device::static_type()),
                discovered_devices: gio::ListStore::with_type(Device::static_type()),
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Devices {
        fn constructed(&self) {
            self.parent_constructed();

            self.registered_devices.connect_items_changed(glib::clone!(
                #[strong(rename_to=devices)]
                self.obj(),
                move |_, position, removed, added| {
                    devices.items_changed(position, removed, added);
                }
            ));
            self.discovered_devices.connect_items_changed(glib::clone!(
                #[strong(rename_to=devices)]
                self.obj(),
                move |_, position, removed, added| {
                    devices.items_changed(
                        position + devices.registered_devices().n_items(),
                        removed,
                        added,
                    );
                }
            ));
        }
    }

    impl ListModelImpl for Devices {
        fn item_type(&self) -> glib::Type {
            Device::static_type()
        }

        fn n_items(&self) -> u32 {
            self.registered_devices.n_items() + self.discovered_devices.n_items()
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            if position < self.registered_devices.n_items() {
                self.registered_devices.item(position)
            } else {
                self.discovered_devices
                    .item(position - self.registered_devices.n_items())
            }
        }
    }
}
