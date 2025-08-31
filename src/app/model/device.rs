// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use std::time::Duration;

use gtk::glib;

use crate::config::G_LOG_DOMAIN;
use crate::futures::future_with_timeout;
use crate::net::{MacAddr6Boxed, SocketAddrBoxed, wol};

glib::wrapper! {
    pub struct Device(ObjectSubclass<imp::Device>);
}

impl Device {
    pub fn new(
        label: &str,
        mac_address: MacAddr6Boxed,
        host: &str,
        target_address: SocketAddrBoxed,
    ) -> Self {
        glib::Object::builder()
            .property("label", label)
            .property("mac_address", mac_address)
            .property("host", host)
            .property("target_address", target_address)
            .build()
    }

    /// Send the magic packet to this device.
    pub async fn wol(&self) -> Result<(), glib::Error> {
        let mac_address = self.mac_address();
        let target_address = self.target_address();
        glib::info!(
            "Sending magic packet for mac address {mac_address} of device {} to {target_address}",
            self.label()
        );
        let wol_timeout = Duration::from_secs(5);
        future_with_timeout(wol_timeout, wol(*mac_address, *target_address))
            .await
            .inspect(|()| {
                glib::info!(
                    "Sent magic packet to {mac_address} of device {} to {target_address}",
                    self.label()
                );
            })
            .inspect_err(|error| {
                glib::warn!(
                    "Failed to send magic packet to {mac_address} of device {} to {target_address}: {error}",
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

    use crate::net::{MacAddr6Boxed, SocketAddrBoxed};

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
        /// The target address to send the magic packet for this device to.
        #[property(get, set)]
        pub target_address: RefCell<SocketAddrBoxed>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Device {
        const NAME: &'static str = "TurnOnDevice";

        type Type = super::Device;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Device {}
}
