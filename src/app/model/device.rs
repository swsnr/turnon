// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::time::Duration;

use gtk::glib;

use crate::config::G_LOG_DOMAIN;
use crate::futures::future_with_timeout;
use crate::net::{MacAddr6Boxed, wol};

glib::wrapper! {
    pub struct Device(ObjectSubclass<imp::Device>);
}

impl Device {
    pub fn new(label: &str, mac_address: MacAddr6Boxed, host: &str) -> Self {
        glib::Object::builder()
            .property("label", label)
            .property("mac_address", mac_address)
            .property("host", host)
            .build()
    }

    /// Send the magic packet to this device.
    pub async fn wol(&self) -> Result<(), glib::Error> {
        let mac_address = self.mac_address();
        glib::info!(
            "Sending magic packet for mac address {mac_address} of device {}",
            self.label()
        );
        let wol_timeout = Duration::from_secs(5);
        future_with_timeout(wol_timeout, wol(*mac_address))
            .await
            .inspect(|()| {
                glib::info!(
                    "Sent magic packet to {mac_address} of device {}",
                    self.label()
                );
            })
            .inspect_err(|error| {
                glib::warn!(
                    "Failed to send magic packet to {mac_address} of device{}: {error}",
                    self.label()
                );
            })
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
    }

    #[glib::derived_properties]
    impl ObjectImpl for Device {}
}
