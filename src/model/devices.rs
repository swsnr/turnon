// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::gio;
use gtk::glib;
use gtk::prelude::ListModelExt;
use gtk::subclass::prelude::ObjectSubclassIsExt;

use crate::storage::StoredDevice;

use super::Device;

glib::wrapper! {
    pub struct Devices(ObjectSubclass<imp::Devices>) @implements gio::ListModel;
}

impl Devices {
    pub fn get(&self, n: usize) -> Option<Device> {
        self.imp().0.borrow().get(n).cloned()
    }

    /// Add a new `device` to the list of devices.
    ///
    /// Signal that the end of the items list changed.
    pub fn add_device(&self, device: &Device) {
        let position = {
            let mut data = self.imp().0.borrow_mut();
            data.push(device.clone());
            data.len() - 1
        };
        self.items_changed(position.try_into().unwrap(), 0, 1);
    }

    /// Find the index of the given `device` in the list of devices.
    fn position(&self, device: &Device) -> Option<usize> {
        self.imp().0.borrow().iter().position(|d| d == device)
    }

    /// Delete the given `device`.
    ///
    /// Then signal that the list changed at the position of the device.
    pub fn delete_device(&self, device: &Device) {
        if let Some(position) = self.position(device) {
            let mut data = self.imp().0.borrow_mut();
            data.remove(position);
            // Drop our mutable borrow before emitting the signal to allow other code to access the device list.
            drop(data);
            self.items_changed(position as u32, 1, 0);
        }
    }

    /// Clear the list and ad all given devices.
    pub fn reset_devices(&self, devices: Vec<Device>) {
        let amount_deleted = {
            let mut data = self.imp().0.borrow_mut();
            let len = data.len();
            data.clear();
            len
        };
        self.items_changed(0, amount_deleted.try_into().unwrap(), 0);
        let amount_added = {
            let mut data = self.imp().0.borrow_mut();
            data.extend_from_slice(&devices);
            devices.len()
        };
        self.items_changed(0, 0, amount_added.try_into().unwrap())
    }
}

impl Default for Devices {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl IntoIterator for &Devices {
    type Item = Device;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.imp().0.borrow().clone().into_iter()
    }
}

impl From<&Devices> for Vec<StoredDevice> {
    fn from(val: &Devices) -> Self {
        val.imp()
            .0
            .borrow()
            .iter()
            .map(StoredDevice::from)
            .collect()
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
