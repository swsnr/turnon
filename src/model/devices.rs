// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::gio;
use gtk::glib;
use gtk::prelude::ListModelExt;
use gtk::subclass::prelude::ObjectSubclassIsExt;

use super::Device;

glib::wrapper! {
    pub struct Devices(ObjectSubclass<imp::Devices>) @implements gio::ListModel;
}

impl Devices {
    pub fn add_device(&self, device: &Device) {
        let position = {
            let mut data = self.imp().0.borrow_mut();
            data.push(device.clone());
            data.len() - 1
        };
        println!("Added new device {:?} at {position}", device.imp());
        self.items_changed(position.try_into().unwrap(), 0, 1);
    }
}

impl Default for Devices {
    fn default() -> Self {
        glib::Object::new()
    }
}

mod imp {
    use std::cell::RefCell;

    use gio::subclass::prelude::*;
    use gtk::gio;
    use gtk::glib;
    use gtk::prelude::Cast;
    use gtk::prelude::StaticType;

    use crate::model::Device;

    #[derive(Default)]
    pub struct Devices(pub RefCell<Vec<Device>>);

    #[glib::object_subclass]
    impl ObjectSubclass for Devices {
        const NAME: &'static str = "Devices";

        type Type = super::Devices;

        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for Devices {}

    impl ListModelImpl for Devices {
        fn item_type(&self) -> glib::Type {
            Device::static_type()
        }

        fn n_items(&self) -> u32 {
            self.0.borrow().len().try_into().unwrap()
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.0
                .borrow()
                .get(position as usize)
                .map(|d| d.clone().upcast())
        }
    }
}
