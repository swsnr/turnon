// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::glib;
use macaddr::MacAddr6;

glib::wrapper! {
    pub struct Device(ObjectSubclass<imp::Device>);
}

impl Device {
    pub fn new_with_generated_id(label: String, mac_address: MacAddr6, host: String) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        glib::Object::builder()
            .property("id", id)
            .property("label", label)
            .property("mac_address", glib::Bytes::from(mac_address.as_bytes()))
            .property("host", host)
            .build()
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
        /// A unique ID for this device.
        ///
        /// This ID is mostly used for storage.
        #[property(get, set, construct_only)]
        pub id: RefCell<String>,
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
                id: Default::default(),
                label: Default::default(),
                mac_address: RefCell::new(glib::Bytes::from_static(&[0; 6])),
                host: Default::default(),
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Device {}
}
