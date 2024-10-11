// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::glib;
use macaddr::MacAddr6;

use crate::services::StoredDevice;

glib::wrapper! {
    pub struct Device(ObjectSubclass<imp::Device>);
}

impl Device {
    pub fn new(label: String, mac_address: MacAddr6, host: String) -> Self {
        glib::Object::builder()
            .property("label", label)
            .property("mac_address", glib::Bytes::from(mac_address.as_bytes()))
            .property("host", host)
            .build()
    }

    pub fn mac_addr6(&self) -> MacAddr6 {
        // We unwrap, because we try very hard to make sure that mac_address
        // contains 6 bytes.
        let data: [u8; 6] = (*self.mac_address()).try_into().unwrap();
        MacAddr6::from(data)
    }
}

impl Default for Device {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl From<StoredDevice> for Device {
    fn from(value: StoredDevice) -> Self {
        glib::Object::builder()
            .property("label", value.label)
            .property(
                "mac_address",
                glib::Bytes::from(value.mac_address.as_bytes()),
            )
            .property("host", value.host)
            .build()
    }
}

impl From<&Device> for StoredDevice {
    fn from(device: &Device) -> Self {
        StoredDevice {
            label: device.label(),
            host: device.host(),
            mac_address: device.mac_addr6(),
        }
    }
}

mod imp {
    use std::cell::RefCell;

    use glib::prelude::*;
    use glib::subclass::prelude::*;
    use gtk::glib;

    #[derive(Debug, glib::Properties)]
    #[properties(wrapper_type = super::Device)]
    pub struct Device {
        /// The human-readable label for this device, for display in the UI.
        #[property(get, set)]
        pub label: RefCell<String>,
        /// The MAC address of the device to wake.
        #[property(get, set)]
        pub mac_address: RefCell<glib::Bytes>,
        /// The host name or IP 4/6 address of the device, to check whether it is reachable.
        #[property(get, set)]
        pub host: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Device {
        const NAME: &'static str = "Device";

        type Type = super::Device;

        fn new() -> Self {
            Self {
                label: Default::default(),
                mac_address: RefCell::new(glib::Bytes::from_static(&[0; 6])),
                host: Default::default(),
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Device {}
}