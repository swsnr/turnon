// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::glib;

use crate::net::MacAddr6Boxed;

use super::WakeableDevice;

glib::wrapper! {
    pub struct Device(ObjectSubclass<imp::Device>) @implements WakeableDevice;
}

impl Device {
    pub fn new(label: &str, mac_address: MacAddr6Boxed, host: &str) -> Self {
        glib::Object::builder()
            .property("label", label)
            .property("mac-address", mac_address)
            .property("host", host)
            .build()
    }
}

impl Default for Device {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

mod imp {
    use std::cell::RefCell;

    use glib::prelude::*;
    use glib::subclass::prelude::*;
    use gtk::glib;

    use crate::app::model::WakeableDevice;
    use crate::net::MacAddr6Boxed;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Device)]
    pub struct Device {
        /// The human-readable label for this device, for display in the UI.
        #[property(get, set)]
        pub label: RefCell<String>,
        /// The MAC address of the device to wake.
        #[property(get, set)]
        pub mac_address: RefCell<MacAddr6Boxed>,
        /// The host name or IP 4/6 address of the device, to check whether it is reachable.
        #[property(get, set)]
        pub host: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Device {
        const NAME: &'static str = "Device";

        type Type = super::Device;

        type Interfaces = (WakeableDevice,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for Device {}
}

#[cfg(test)]
mod tests {
    use glib::object::Cast;

    use crate::app::model::{WakeableDevice, WakeableDeviceExt};

    use super::Device;

    #[test]
    fn test_property_in_interface() {
        let device = Device::default();
        device.set_label("foobar");

        let device_iface: WakeableDevice = device.upcast();
        assert_eq!(device_iface.label(), "foobar");
    }
}
